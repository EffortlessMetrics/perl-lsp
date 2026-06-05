#!/usr/bin/env bash
# Mid-cycle worktree cleanup for swarm agents.
#
# Problem: worktree-isolated agents leave worktrees behind after completing.
# During a cycle with 20+ agents, worktrees accumulate, consuming disk and
# polluting `git worktree list`.
#
# This script intelligently cleans up based on merge/PR state:
#   - Merged branches     → remove worktree + delete branch
#   - Open PR + passing CI → leave (might need fixups)
#   - No PR + no unpushed → remove (abandoned)
#   - No PR + unpushed    → leave (work in progress)
#
# Usage:
#   bash scripts/cleanup-completed-worktrees.sh [--dry-run]
#
# Optional:
#   CLEANUP_BASE_BRANCH=<branch>  branch used for merged/unpushed comparisons

set -euo pipefail

DRY_RUN=false
if [[ "${1:-}" == "--dry-run" ]]; then
    DRY_RUN=true
fi

CLEANUP_BASE_BRANCH="${CLEANUP_BASE_BRANCH:-main}"

# Navigate to the repo root (main worktree)
REPO_ROOT="$(git rev-parse --path-format=absolute --git-common-dir | sed 's|/.git$||')"

echo "=== Mid-Cycle Worktree Cleanup ==="
echo "Repo root: $REPO_ROOT"
echo "Base branch: $CLEANUP_BASE_BRANCH"
echo "Dry run: $DRY_RUN"
echo ""

# Prune references to already-deleted worktree directories
git worktree prune

# Counters
REMOVED=0
KEPT=0
SKIPPED=0

# Collect worktree paths under .claude/worktrees/
WORKTREES=()
while IFS= read -r line; do
    wt_path="$(echo "$line" | awk '{print $1}')"
    # Only consider worktrees under .claude/worktrees/
    if [[ "$wt_path" == *"/.claude/worktrees/"* ]]; then
        WORKTREES+=("$line")
    fi
done < <(git worktree list)

if [[ ${#WORKTREES[@]} -eq 0 ]]; then
    echo "No agent worktrees found. Nothing to clean."
    exit 0
fi

echo "Found ${#WORKTREES[@]} agent worktrees"
echo ""
printf "%-50s %-35s %-15s %s\n" "WORKTREE" "BRANCH" "STATE" "ACTION"
printf "%-50s %-35s %-15s %s\n" "--------" "------" "-----" "------"

for line in "${WORKTREES[@]}"; do
    wt_path="$(echo "$line" | awk '{print $1}')"
    wt_branch="$(echo "$line" | awk '{print $3}' | tr -d '[]')"

    # Skip detached HEAD worktrees
    if [[ -z "$wt_branch" || "$wt_branch" == "(detached" ]]; then
        printf "%-50s %-35s %-15s %s\n" "$(basename "$wt_path")" "(detached)" "unknown" "SKIP"
        SKIPPED=$((SKIPPED + 1))
        continue
    fi

    # Skip the current worktree (don't clean ourselves)
    CURRENT_BRANCH="$(git rev-parse --abbrev-ref HEAD 2>/dev/null || echo "")"
    if [[ "$wt_branch" == "$CURRENT_BRANCH" ]]; then
        printf "%-50s %-35s %-15s %s\n" "$(basename "$wt_path")" "$wt_branch" "current" "SKIP"
        SKIPPED=$((SKIPPED + 1))
        continue
    fi

    # Check 1: Is the branch merged to the configured base branch?
    MERGED=false
    if git branch --merged "$CLEANUP_BASE_BRANCH" 2>/dev/null | grep -qw "$wt_branch"; then
        MERGED=true
    fi

    if $MERGED; then
        printf "%-50s %-35s %-15s %s\n" "$(basename "$wt_path")" "$wt_branch" "merged" "REMOVE"
        if ! $DRY_RUN; then
            git worktree remove --force "$wt_path" 2>/dev/null || rm -rf "$wt_path"
            git branch -D "$wt_branch" 2>/dev/null || true
        fi
        REMOVED=$((REMOVED + 1))
        continue
    fi

    # Check 2: Does the branch have an open PR?
    HAS_PR=false
    PR_NUMBER=""
    if command -v gh &>/dev/null; then
        PR_NUMBER="$(gh pr list --head "$wt_branch" --state open --json number --jq '.[0].number' 2>/dev/null || echo "")"
        if [[ -n "$PR_NUMBER" ]]; then
            HAS_PR=true
        fi
    fi

    if $HAS_PR; then
        printf "%-50s %-35s %-15s %s\n" "$(basename "$wt_path")" "$wt_branch" "open-pr:#$PR_NUMBER" "KEEP"
        KEPT=$((KEPT + 1))
        continue
    fi

    # Check 3: No PR — does the branch have unpushed commits?
    UNPUSHED=false
    # Check if the branch has a remote tracking branch
    REMOTE_BRANCH="$(git config --get "branch.$wt_branch.merge" 2>/dev/null || echo "")"
    if [[ -z "$REMOTE_BRANCH" ]]; then
        # No tracking branch — check if there are commits beyond the base branch.
        AHEAD="$(git rev-list "$CLEANUP_BASE_BRANCH..$wt_branch" --count 2>/dev/null || echo "0")"
        if [[ "$AHEAD" -gt 0 ]]; then
            UNPUSHED=true
        fi
    else
        # Has tracking branch — check if ahead of remote
        REMOTE_REF="$(echo "$REMOTE_BRANCH" | sed 's|^refs/heads/|origin/|')"
        AHEAD="$(git rev-list "${REMOTE_REF}..$wt_branch" --count 2>/dev/null || echo "0")"
        if [[ "$AHEAD" -gt 0 ]]; then
            UNPUSHED=true
        fi
    fi

    # Also check for uncommitted changes in the worktree
    HAS_DIRTY=false
    if [[ -d "$wt_path" ]]; then
        DIRTY="$(git -C "$wt_path" status --porcelain 2>/dev/null || echo "")"
        if [[ -n "$DIRTY" ]]; then
            HAS_DIRTY=true
        fi
    fi

    if $HAS_DIRTY; then
        printf "%-50s %-35s %-15s %s\n" "$(basename "$wt_path")" "$wt_branch" "dirty" "KEEP"
        KEPT=$((KEPT + 1))
        continue
    fi

    if $UNPUSHED; then
        printf "%-50s %-35s %-15s %s\n" "$(basename "$wt_path")" "$wt_branch" "unpushed" "KEEP"
        KEPT=$((KEPT + 1))
        continue
    fi

    # No PR, no unpushed commits, not dirty — this is abandoned
    printf "%-50s %-35s %-15s %s\n" "$(basename "$wt_path")" "$wt_branch" "abandoned" "REMOVE"
    if ! $DRY_RUN; then
        git worktree remove --force "$wt_path" 2>/dev/null || rm -rf "$wt_path"
        git branch -D "$wt_branch" 2>/dev/null || true
    fi
    REMOVED=$((REMOVED + 1))
done

# Final prune
if ! $DRY_RUN; then
    git worktree prune
fi

echo ""
echo "=== Summary ==="
echo "Removed: $REMOVED"
echo "Kept:    $KEPT"
echo "Skipped: $SKIPPED"
echo "Total:   ${#WORKTREES[@]}"

if $DRY_RUN; then
    echo ""
    echo "(Dry run — no changes made. Remove --dry-run to execute.)"
fi
