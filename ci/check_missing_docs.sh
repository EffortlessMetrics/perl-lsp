#!/bin/bash
# CI script to enforce that perl-parser missing_docs warnings only go down.
# Issue #197: baseline ratchet with JSON-based counting for accuracy.
#
# This script:
# - Uses `cargo check --message-format=json` for reliable warning counting
# - Includes `--tests` to catch undocumented test-only public APIs
# - Filters to perl-parser only (ignores dependencies)
# - Enforces a baseline ratchet: count can only go down, never up

set -euo pipefail

BASELINE_FILE="ci/missing_docs_baseline.txt"

if [[ ! -f "$BASELINE_FILE" ]]; then
  echo "Baseline file not found: $BASELINE_FILE"
  exit 2
fi

baseline="$(cat "$BASELINE_FILE" | tr -d '[:space:]')"
if [[ -z "$baseline" ]]; then
  echo "Baseline file is empty: $BASELINE_FILE"
  exit 2
fi

# Count missing_docs warnings using JSON output for accuracy
# This avoids grep-based parsing issues and ensures we only count perl-parser warnings
count_missing_docs() {
  # Check if python3 is available, fall back to python
  local python_cmd="python3"
  if ! command -v python3 &> /dev/null; then
    if command -v python &> /dev/null; then
      python_cmd="python"
    else
      echo "Error: Python is required for JSON parsing" >&2
      exit 2
    fi
  fi

  cargo check -p perl-parser --tests --message-format=json 2>/dev/null \
    | $python_cmd -c '
import json
import sys

count = 0
for line in sys.stdin:
    line = line.strip()
    if not line:
        continue
    try:
        obj = json.loads(line)
    except json.JSONDecodeError:
        continue

    # Only count compiler messages
    if obj.get("reason") != "compiler-message":
        continue

    # Only count messages from perl-parser package
    pkg_id = obj.get("package_id", "")
    if not pkg_id.startswith("perl-parser "):
        continue

    # Extract the message object
    msg = obj.get("message", {})
    if not msg:
        continue

    # Only count warnings with missing_docs code
    level = msg.get("level")
    code = (msg.get("code") or {}).get("code")

    if level == "warning" and code == "missing_docs":
        count += 1

print(count)
'
}

current=$(count_missing_docs)

echo "Missing docs warnings (perl-parser, tests included): $current"
echo "Baseline: $baseline"

if [[ "$current" -gt "$baseline" ]]; then
  echo ""
  echo "REGRESSION: missing_docs count increased from $baseline to $current"
  echo ""
  echo "To see the warnings, run:"
  echo "  cargo check -p perl-parser --tests 2>&1 | grep 'missing documentation'"
  echo ""
  echo "Options:"
  echo "  1. Add documentation to the new public items"
  echo "  2. Mark test-only items with #[doc(hidden)] (still requires docs)"
  echo "  3. If intentional, update baseline: echo $current > $BASELINE_FILE"
  exit 1
fi

if [[ "$current" -lt "$baseline" ]]; then
  echo ""
  echo "IMPROVEMENT: $((baseline - current)) fewer missing_docs warnings!"
  echo "Consider updating baseline: echo $current > $BASELINE_FILE"
fi

echo ""
echo "Check passed: missing_docs count is within acceptable range"
exit 0
