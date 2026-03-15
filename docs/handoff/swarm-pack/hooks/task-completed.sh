#!/bin/bash
# TaskCompleted hook: quality gate before allowing task completion
# Exit 2 = reject with feedback
# Exit 0 = allow

# Replace with your project's format check command
FMT_CHECK_CMD="${FMT_CHECK_CMD:-cargo fmt --all -- --check}"

if ! eval "$FMT_CHECK_CMD" 2>/dev/null; then
  echo "Task completion blocked: format check failed. Fix formatting before marking complete."
  exit 2
fi

exit 0
