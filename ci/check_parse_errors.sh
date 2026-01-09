#!/bin/bash
# CI script to enforce that parse error count only goes down.
# Issue #180: baseline ratchet for parser feature coverage.
#
# This script:
# - Runs corpus audit to count parse errors in test corpus
# - Enforces a baseline ratchet: count can only go down, never up
# - Outputs actionable guidance for regressions or improvements

set -euo pipefail

BASELINE_FILE="ci/parse_errors_baseline.txt"
REPORT_FILE="corpus_audit_report.json"

if [[ ! -f "$BASELINE_FILE" ]]; then
  echo "Baseline file not found: $BASELINE_FILE"
  exit 2
fi

baseline="$(cat "$BASELINE_FILE" | tr -d '[:space:]')"
if [[ -z "$baseline" ]]; then
  echo "Baseline file is empty: $BASELINE_FILE"
  exit 2
fi

# Run corpus audit to generate fresh report
echo "Running corpus audit..."
cargo run -p xtask --no-default-features -q -- corpus-audit --fresh --corpus-path . --output "$REPORT_FILE" 2>/dev/null || true

if [[ ! -f "$REPORT_FILE" ]]; then
  echo "Report file not generated: $REPORT_FILE"
  exit 2
fi

# Extract error count from JSON report
extract_error_count() {
  # Check if python3 is available, fall back to python
  local python_cmd="python3"
  if ! command -v python3 &> /dev/null; then
    if command -v python &> /dev/null; then
      python_cmd="python"
    else
      # Try jq as fallback
      if command -v jq &> /dev/null; then
        jq -r '.parse_outcomes.error' "$REPORT_FILE"
        return
      fi
      echo "Error: Python or jq is required for JSON parsing" >&2
      exit 2
    fi
  fi

  $python_cmd -c "
import json
import sys

try:
    with open('$REPORT_FILE') as f:
        report = json.load(f)
    print(report['parse_outcomes']['error'])
except Exception as e:
    print(f'Error reading report: {e}', file=sys.stderr)
    sys.exit(2)
"
}

current=$(extract_error_count)

echo ""
echo "Parse errors in test corpus: $current"
echo "Baseline: $baseline"

if [[ "$current" -gt "$baseline" ]]; then
  echo ""
  echo "REGRESSION: parse error count increased from $baseline to $current"
  echo ""
  echo "To see details, run:"
  echo "  just parser-audit"
  echo ""
  echo "Options:"
  echo "  1. Fix the parser to handle the new failing constructs"
  echo "  2. If the regression is intentional, update baseline: echo $current > $BASELINE_FILE"
  exit 1
fi

if [[ "$current" -lt "$baseline" ]]; then
  echo ""
  echo "IMPROVEMENT: $((baseline - current)) fewer parse errors!"
  echo "Consider updating baseline: echo $current > $BASELINE_FILE"
fi

echo ""
echo "Check passed: parse error count is within acceptable range"
exit 0
