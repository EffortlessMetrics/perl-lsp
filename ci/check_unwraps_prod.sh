#!/bin/bash
# CI hygiene metric: track unwrap/expect count in crates/*/src/ directories
# NOTE: This is a hygiene ratchet, NOT a production safety gate.
# - Includes inline tests (#[cfg(test)]) within src/ directories
# - Production safety is enforced by: just clippy-prod-no-unwrap
# Enhanced with top offenders and diff analysis for actionable feedback
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"

# Patterns for panic-prone calls:
# - .unwrap() is always a panic risk
# - .expect("...") with string literal is panic-prone (not domain methods like self.expect(TokenKind::...))
# - .expect(format!(...)) with formatted message is panic-prone
PATTERN_UNWRAP='\.unwrap\(\)'
PATTERN_EXPECT_STR='\.expect\(\s*"'
PATTERN_EXPECT_FMT='\.expect\(\s*&?format!\('
# Combined for single-pass counting
PATTERN="$PATTERN_UNWRAP|$PATTERN_EXPECT_STR|$PATTERN_EXPECT_FMT"

count_unwraps() {
  local search_path="$1"
  if command -v rg &>/dev/null; then
    # Use ripgrep for fast, accurate counting
    # Count .unwrap( and .expect( patterns across all matching files
    # Exclude safe patterns like unwrap_or, unwrap_or_else, unwrap_or_default
    # Exclude tree-sitter-perl-rs (not in production workspace)
    local count=0
    for dir in crates/*/src; do
      # Skip tree-sitter-perl-rs (excluded from workspace, not in production)
      if [[ "$dir" == "crates/tree-sitter-perl-rs/src" ]]; then
        continue
      fi
      # Skip perl-parser-pest (legacy v2 implementation, not in production)
      if [[ "$dir" == "crates/perl-parser-pest/src" ]]; then
        continue
      fi
      if [ -d "$dir" ]; then
        dir_count=$(rg "$PATTERN" "$dir" --count-matches 2>/dev/null | \
          awk -F: '{sum+=$2} END {print sum+0}')
        count=$((count + dir_count))
      fi
    done
    echo "$count"
  else
    # Fallback to grep (portable but slower)
    # Exclude tree-sitter-perl-rs (not in production workspace)
    find crates/*/src -name "*.rs" -type f ! -path "crates/tree-sitter-perl-rs/*" \
      -exec grep -h '\.unwrap(\|\.expect(' {} \; 2>/dev/null | \
      wc -l | awk '{print $1+0}'
  fi
}

show_top_offenders() {
  echo "Top offenders (by file):"
  if command -v rg &>/dev/null; then
    # Exclude tree-sitter-perl-rs (not in production workspace)
    rg "$PATTERN" crates/*/src -c 2>/dev/null | \
      grep -v "^crates/tree-sitter-perl-rs/" | \
      grep -v "^crates/perl-parser-pest/" | \
      sort -t: -k2 -rn | head -15 | \
      awk -F: '{printf "  %4d  %s\n", $2, $1}'
  fi
  echo ""
}

show_new_unwraps() {
  # Show newly introduced unwrap/expect in this branch relative to base
  local base_branch
  base_branch="$(git symbolic-ref refs/remotes/origin/HEAD 2>/dev/null | sed 's@^refs/remotes/origin/@@' || echo 'master')"

  local merge_base
  merge_base="$(git merge-base HEAD "origin/$base_branch" 2>/dev/null || git merge-base HEAD "$base_branch" 2>/dev/null || echo '')"

  if [ -n "$merge_base" ]; then
    echo "New unwrap/expect lines since merge-base (${merge_base:0:8}):"
    local new_lines
    new_lines=$(git diff -U0 "$merge_base"..HEAD -- '*.rs' 2>/dev/null | \
      rg "^\+[^+].*($PATTERN)" 2>/dev/null || true)
    if [ -n "$new_lines" ]; then
      echo "$new_lines" | head -20
      local count
      count=$(echo "$new_lines" | wc -l)
      if [ "$count" -gt 20 ]; then
        echo "  ... and $((count - 20)) more"
      fi
    else
      echo "  (none)"
    fi
    echo ""
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

echo "Hygiene unwrap/expect count (includes inline tests in src/): $current (baseline: $baseline)"
echo ""

# Show top offenders for visibility
show_top_offenders

# Show new unwraps introduced in this branch
show_new_unwraps

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
    echo "✅ IMPROVEMENT: hygiene count decreased by $reduction"
    echo "   Consider updating baseline: echo $current > ci/unwrap_prod_baseline.txt"
  else
    echo "✅ PASS: hygiene count maintained at baseline"
  fi
  echo ""
  echo "Hygiene ratchet passed (includes inline tests in src/)"
  echo "NOTE: Production safety enforced separately by: just clippy-prod-no-unwrap"
  exit 0
else
  echo "❌ REGRESSION: hygiene count increased from $baseline to $current"
  echo "   New unwraps added: $((current - baseline))"
  echo ""
  echo "WARNING: Unwrap count in src/ directories has increased"
  echo "Action required:"
  echo "  1. Replace new .unwrap() and .expect() calls with proper error handling"
  echo "  2. Use Result<T, E> propagation with ? operator"
  echo "  3. Consider using unwrap_or_default() or map_err() for safer alternatives"
  echo ""
  echo "To see recent unwraps:"
  echo "  rg '\\.unwrap\\(\\)|\\.expect\\(\\s*\"' crates/*/src -n"
  exit 1
fi
