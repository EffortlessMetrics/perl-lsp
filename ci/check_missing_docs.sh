#!/bin/bash
# CI script to enforce that the missing_docs warning count only goes down
# Follows the same ratcheting pattern as check_ignored.sh (Issue #197)
set -euo pipefail

# Count missing_docs warnings by building with the warning enabled
count_missing_docs() {
  # Build perl-parser with missing_docs warning and count warnings
  # Use 2>&1 to capture stderr where warnings go
  local count
  count=$(RUSTFLAGS="-W missing_docs" cargo build -p perl-parser 2>&1 | grep -c "warning: missing documentation" || echo "0")
  echo "$count"
}

current=$(count_missing_docs)

baseline_file="ci/missing_docs_baseline.txt"
if [ -f "$baseline_file" ]; then
  baseline=$(cat "$baseline_file")
else
  baseline=$current
  echo "$baseline" > "$baseline_file"
  echo "Created baseline file with count: $baseline"
fi

# Target for v0.9.0: significantly reduce missing docs
target=100

echo "Missing documentation warnings: $current (baseline: $baseline)"
echo ""
echo "Budget Analysis:"
echo "  - Target: ≤$target warnings"
echo "  - Current baseline: $baseline warnings"
echo "  - Reduction needed: $((baseline - target)) warnings"

if [ "$current" -le "$target" ]; then
  reduction=$((baseline - current))
  echo "  ✅ TARGET ACHIEVED: $current ≤ $target"
  echo "  📈 Total reduction: $reduction warnings from baseline"
elif [ "$current" -le "$baseline" ]; then
  echo "  🔄 PROGRESS: $current ≤ $baseline (baseline maintained)"
  echo "  ⚠️  Need $((current - target)) more reductions to reach target"
else
  echo "  ❌ REGRESSION: $current > $baseline"
fi

echo ""
if [ "$current" -le "$baseline" ]; then
  echo "Check passed: missing_docs count is within acceptable range"
  exit 0
else
  echo "ERROR: Missing docs count has increased from $baseline to $current"
  echo "Please document new public items or update the baseline if intentional"
  exit 1
fi
