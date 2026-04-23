#!/usr/bin/env bash
# CI check: verify all registered hook scripts are executable
#
# Usage:
#   ./.ci/scripts/check-hook-executable.sh
#
# Exit codes:
#   0 — all hook scripts are executable
#   1 — one or more hook scripts lack the executable bit

set -euo pipefail

FAILED=0

for f in .claude/hooks/*.sh; do
  if [[ ! -x "$f" ]]; then
    echo "::error::Hook not executable: $f" >&2
    FAILED=1
  fi
done

if [[ "$FAILED" -eq 0 ]]; then
  echo "Hook executable check passed"
fi

exit "$FAILED"
