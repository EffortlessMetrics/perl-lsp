#!/usr/bin/env bash
# Clean up stale agent worktrees from previous sessions
# Safe: only removes worktrees under .claude/worktrees/ that are not the current session

set -euo pipefail

echo "=== Worktree Cleanup ==="

# Prune references to deleted worktrees
git worktree prune

# Count current worktrees (excluding main and /tmp)
STALE=$(git worktree list | grep -c '.claude/worktrees/' || echo 0)
echo "Found $STALE agent worktrees"

if [[ "$STALE" -eq 0 ]]; then
    echo "No stale worktrees to clean up"
    exit 0
fi

# Remove each worktree
git worktree list | grep '.claude/worktrees/' | awk '{print $1}' | while read -r wt; do
    echo "Removing: $wt"
    git worktree remove --force "$wt" 2>/dev/null || rm -rf "$wt"
done

# Final prune
git worktree prune
echo "Cleanup complete"
