#!/bin/bash
# CI ratchet gate: enforce that production unwrap/expect count only goes down
set -euo pipefail

count_unwraps() {
  local search_path="$1"
  if command -v rg &>/dev/null; then
    # Use ripgrep for fast, accurate counting
    # Count .unwrap( and .expect( patterns across all matching files
    # We need to expand the glob pattern first for ripgrep
    local count=0
    for dir in crates/*/src; do
      if [ -d "$dir" ]; then
        dir_count=$(rg '\.unwrap\(|\.expect\(' "$dir" --count-matches 2>/dev/null | awk -F: '{sum+=$2} END {print sum+0}')
        count=$((count + dir_count))
      fi
    done
    echo "$count"
  else
    # Fallback to grep (portable but slower)
    find crates/*/src -name "*.rs" -type f -exec grep -h '\.unwrap(\|\.expect(' {} \; 2>/dev/null | \
      wc -l | awk '{print $1+0}'
  fi
}

# Count unwraps in production code (src/ directories only)
current=$(count_unwraps "crates/*/src")

baseline_file="ci/unwrap_prod_baseline.txt"
if [ -f "$baseline_file" ]; then
  baseline=$(cat "$baseline_file")
else
  baseline=$current
  echo "$baseline" > "$baseline_file"
  echo "Created baseline file with count: $baseline"
fi

# Calculate reduction metrics
reduction=$((baseline - current))
reduction_percent=0
if [ "$baseline" -gt 0 ]; then
  reduction_percent=$(( (reduction * 100) / baseline ))
fi

echo "Production unwrap/expect count: $current (baseline: $baseline)"
echo ""
echo "Ratchet Analysis:"
echo "  - Current count: $current"
echo "  - Baseline: $baseline"
echo "  - Change: $reduction"

if [ "$reduction" -gt 0 ]; then
  echo "  - Reduction: $reduction_percent%"
fi

echo ""

if [ "$current" -le "$baseline" ]; then
  if [ "$current" -lt "$baseline" ]; then
    echo "✅ IMPROVEMENT: unwrap count decreased by $reduction"
    echo "   Consider updating baseline: echo $current > ci/unwrap_prod_baseline.txt"
  else
    echo "✅ PASS: unwrap count maintained at baseline"
  fi
  echo ""
  echo "Ratchet gate passed: production unwrap count is within acceptable range"
  exit 0
else
  echo "❌ REGRESSION: unwrap count increased from $baseline to $current"
  echo "   New unwraps added: $((current - baseline))"
  echo ""
  echo "ERROR: Production unwrap count has increased"
  echo "Action required:"
  echo "  1. Replace new .unwrap() and .expect() calls with proper error handling"
  echo "  2. Use Result<T, E> propagation with ? operator"
  echo "  3. Consider using unwrap_or_default() or map_err() for safer alternatives"
  echo ""
  echo "To see recent unwraps:"
  echo "  rg '\\.unwrap\\(|\\.expect\\(' crates/*/src -n"
  exit 1
fi
