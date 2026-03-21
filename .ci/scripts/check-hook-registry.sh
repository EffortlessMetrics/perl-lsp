#!/usr/bin/env bash
# CI check: verify hook registry in settings.json matches files on disk
#
# Parses .claude/settings.json, extracts every .command that ends in .sh,
# strips the "$CLAUDE_PROJECT_DIR"/ prefix, and verifies each file exists
# and is executable.
#
# Usage:
#   ./.ci/scripts/check-hook-registry.sh
#
# Exit codes:
#   0 — all registered hook scripts exist and are executable
#   1 — one or more registered scripts are missing or not executable

set -euo pipefail

SETTINGS=".claude/settings.json"
FAILED=0

if [[ ! -f "$SETTINGS" ]]; then
  echo "::error::$SETTINGS not found" >&2
  exit 1
fi

if ! command -v jq &>/dev/null; then
  echo "::error::jq is required but not installed" >&2
  exit 1
fi

# Extract all .sh command paths from the hooks section
# Strip the "$CLAUDE_PROJECT_DIR"/ prefix (with or without quotes around the variable)
mapfile -t SCRIPT_PATHS < <(
  jq -r '
    .hooks
    | to_entries[]
    | .value[]
    | .hooks[]?
    | .command
    | select(endswith(".sh"))
  ' "$SETTINGS" \
  | sed 's|"\$CLAUDE_PROJECT_DIR"/||g' \
  | sed "s|\\\$CLAUDE_PROJECT_DIR/||g"
)

if [[ "${#SCRIPT_PATHS[@]}" -eq 0 ]]; then
  echo "No .sh hook scripts registered in $SETTINGS — nothing to check"
  exit 0
fi

for rel_path in "${SCRIPT_PATHS[@]}"; do
  if [[ ! -f "$rel_path" ]]; then
    echo "::error::Registered hook script missing: $rel_path" >&2
    FAILED=1
  elif [[ ! -x "$rel_path" ]]; then
    echo "::error::Registered hook script not executable: $rel_path" >&2
    FAILED=1
  else
    echo "  OK: $rel_path"
  fi
done

if [[ "$FAILED" -eq 0 ]]; then
  echo "Hook registry check passed (${#SCRIPT_PATHS[@]} scripts verified)"
fi

exit "$FAILED"
