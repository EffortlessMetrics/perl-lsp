#!/bin/bash
# TaskCompleted hook: verify quality before allowing task completion
# Exit 2 = reject completion with feedback
# Exit 0 = allow completion

# Quick sanity check: is cargo fmt clean?
if ! cargo fmt --all -- --check 2>/dev/null; then
  echo "Task completion blocked: cargo fmt check failed. Run 'cargo fmt --all' before marking complete."
  exit 2
fi

# Check if test files were modified and CURRENT_STATUS.md needs updating
REPO_ROOT="$(git rev-parse --show-toplevel 2>/dev/null || echo ".")"
if git diff --cached --name-only 2>/dev/null | grep -qE '^crates/.*/tests/.*\.rs$' || \
   git diff --name-only HEAD~1 2>/dev/null | grep -qE '^crates/.*/tests/.*\.rs$'; then
  if command -v python3 &>/dev/null && [ -f "$REPO_ROOT/scripts/update-current-status.py" ]; then
    python3 "$REPO_ROOT/scripts/update-current-status.py" 2>/dev/null || true
    if ! git diff --quiet -- docs/project/CURRENT_STATUS.md 2>/dev/null; then
      echo "Task completion blocked: test files changed but CURRENT_STATUS.md has stale counts."
      echo "Run: python3 scripts/update-current-status.py && git add docs/project/CURRENT_STATUS.md"
      exit 2
    fi
  fi
fi

# Allow completion
exit 0
