#!/bin/bash
# CI ratchet gates for critical module scopes
# These have tighter baselines than the global ratchet
set -euo pipefail

# Patterns for panic-prone calls:
# - .unwrap() is always a panic risk
# - .expect("...") with string literal is panic-prone (not domain methods like self.expect(TokenKind::...))
# - .expect(format!(...)) with formatted message is panic-prone
PATTERN_UNWRAP='\.unwrap\(\)'
PATTERN_EXPECT_STR='\.expect\(\s*"'
PATTERN_EXPECT_FMT='\.expect\(\s*&?format!\('
PATTERN="$PATTERN_UNWRAP|$PATTERN_EXPECT_STR|$PATTERN_EXPECT_FMT"

# Count unwraps in production code only (exclude test modules)
# We use line-number-aware filtering to exclude #[cfg(test)] sections
count_prod_unwraps() {
  local dir="$1"
  local test_start_pattern="$2"

  if [ ! -d "$dir" ]; then
    echo "0"
    return
  fi

  if command -v rg &>/dev/null; then
    # Find all unwrap/expect with line numbers
    local total=0
    while IFS= read -r file; do
      # Get the line number where #[cfg(test)] starts (or a very high number if not found)
      local test_start
      test_start=$(rg -n '#\[cfg\(test\)\]' "$file" 2>/dev/null | head -1 | cut -d: -f1 || echo "999999")
      test_start=${test_start:-999999}

      # Count unwraps only before the test section
      local count
      count=$(rg -n "$PATTERN" "$file" 2>/dev/null | while IFS=: read -r line rest; do
        if [ "$line" -lt "$test_start" ]; then
          echo "1"
        fi
      done | wc -l)
      total=$((total + count))
    done < <(find "$dir" -name "*.rs" -type f 2>/dev/null)
    echo "$total"
  else
    # Fallback: count all (less accurate)
    find "$dir" -name "*.rs" -type f -exec grep -h "$PATTERN" {} \; 2>/dev/null | wc -l | awk '{print $1+0}'
  fi
}

check_module() {
  local name="$1"
  local dir="$2"
  local baseline_file="$3"

  echo "=== Checking $name ==="

  if [ ! -d "$dir" ]; then
    echo "  Directory not found: $dir (skipping)"
    echo ""
    return 0
  fi

  local current
  current=$(count_prod_unwraps "$dir" "#\[cfg\(test\)\]")

  local baseline
  if [ -f "$baseline_file" ]; then
    baseline=$(cat "$baseline_file")
  else
    baseline="$current"
    echo "$baseline" > "$baseline_file"
    echo "  Created baseline: $baseline"
  fi

  echo "  Current: $current (baseline: $baseline)"

  if [ "$current" -le "$baseline" ]; then
    if [ "$current" -lt "$baseline" ]; then
      echo "  ✅ IMPROVED by $((baseline - current))"
      echo "  Consider updating: echo $current > $baseline_file"
    else
      echo "  ✅ PASS"
    fi
    echo ""
    return 0
  else
    echo "  ❌ REGRESSION: +$((current - baseline))"
    rg "$PATTERN" "$dir" -n --no-heading 2>/dev/null | head -10
    echo ""
    return 1
  fi
}

echo "Module-scoped unwrap ratchet gates"
echo "==================================="
echo ""

failures=0

# Check server_impl (P0 critical - user-facing LSP handlers)
check_module "server_impl (P0)" \
  "crates/perl-parser/src/lsp/server_impl" \
  "ci/unwrap_server_impl_baseline.txt" || failures=$((failures + 1))

# Check lexer (P1 critical - user input processing)
check_module "lexer (P1)" \
  "crates/perl-lexer/src" \
  "ci/unwrap_lexer_baseline.txt" || failures=$((failures + 1))

if [ "$failures" -gt 0 ]; then
  echo "❌ $failures module ratchet(s) failed"
  exit 1
else
  echo "✅ All module ratchets passed"
  exit 0
fi
