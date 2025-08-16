#!/bin/bash
# CI script to enforce that the ignored test count only goes down
set -euo pipefail

count_ignores () {
  if command -v rg &>/dev/null; then
    rg "^\s*#\[ignore\b" "$1" --count-matches 2>/dev/null | awk -F: '{sum+=$2} END {print sum+0}'
  else
    # Fallback: crude but portable
    grep -R "^[[:space:]]*#\[ignore" "$1" 2>/dev/null | wc -l | awk '{print $1+0}'
  fi
}

# Count in both integration and unit test locations
current_tests=$(count_ignores crates/perl-parser/tests)
current_src=$(count_ignores crates/perl-parser/src)
current=$(( current_tests + current_src ))

baseline_file="ci/ignored_baseline.txt"
if [ -f "$baseline_file" ]; then
  baseline=$(cat "$baseline_file")
else
  baseline=$current
  echo "$baseline" > "$baseline_file"
  echo "Created baseline file with count: $baseline"
fi

echo "Ignored tests: $current (baseline: $baseline)"
echo "  - Integration tests: $current_tests"
echo "  - Unit tests in src: $current_src"

if [ "$current" -le "$baseline" ]; then
  echo "Check passed: ignored test count is within acceptable range"
  exit 0
else
  echo "ERROR: Ignored test count has increased from $baseline to $current"
  echo "Please fix the newly ignored tests or update the baseline if this is intentional"
  exit 1
fi