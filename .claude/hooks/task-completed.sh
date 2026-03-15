#!/bin/bash
# TaskCompleted hook: verify quality before allowing task completion
# Exit 2 = reject completion with feedback
# Exit 0 = allow completion

# Quick sanity check: is cargo fmt clean?
if ! cargo fmt --all -- --check 2>/dev/null; then
  echo "Task completion blocked: cargo fmt check failed. Run 'cargo fmt --all' before marking complete."
  exit 2
fi

# Allow completion
exit 0
