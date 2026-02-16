#!/usr/bin/env bash
set -euo pipefail

UNWRAP_PATTERN='\.unwrap\(|\.expect\('
PANIC_FAMILY_PATTERN='^(?!\s*//).*(panic!\(|todo!\(|unimplemented!\(|unreachable!\()'

tmp="$(mktemp)"
tmp_panic="$(mktemp)"
tmp_files="$(mktemp)"
trap 'rm -f "$tmp" "$tmp_panic" "$tmp_files"' EXIT

# Build source file list, excluding legacy/support crates and test-only files.
find crates -path '*/src/*.rs' -type f \
  ! -path '*/tree-sitter-perl-rs/*' \
  ! -path '*/tree-sitter-perl-c/*' \
  ! -path '*/perl-parser-pest/*' \
  ! -path '*/perl-tdd-support/*' \
  ! -path '*/tests/*' \
  ! -name '*_test.rs' \
  ! -name '*_tests.rs' \
  ! -name 'tests.rs' >"$tmp_files"

unwrap_count=0
panic_count=0
>"$tmp"
>"$tmp_panic"

while IFS= read -r file; do
  # Ignore inline test modules declared after `#[cfg(test)]`.
  test_start=$(rg -n '^\s*#\[cfg\(test\)\]' "$file" | head -1 | cut -d: -f1 || true)
  [[ -z "$test_start" ]] && test_start=999999

  unwrap_matches=$(rg -nH "$UNWRAP_PATTERN" "$file" || true)
  if [[ -n "$unwrap_matches" ]]; then
    while IFS=: read -r matched_file line text; do
      if (( line < test_start )) && ! echo "$text" | rg -q "(self|s|self\\.context)\\.expect\\("; then
        echo "$matched_file:$line:$text" >>"$tmp"
        unwrap_count=$((unwrap_count + 1))
      fi
    done <<<"$unwrap_matches"
  fi

  panic_matches=$(rg --pcre2 -nH "$PANIC_FAMILY_PATTERN" "$file" || true)
  if [[ -n "$panic_matches" ]]; then
    while IFS=: read -r matched_file line text; do
      if (( line < test_start )); then
        echo "$matched_file:$line:$text" >>"$tmp_panic"
        panic_count=$((panic_count + 1))
      fi
    done <<<"$panic_matches"
  fi
done <"$tmp_files"

baseline_file="ci/unwrap_prod_baseline.txt"
if [ -f "$baseline_file" ]; then
  baseline=$(cat "$baseline_file")
else
  baseline=0
fi

echo "unwrap/expect: $unwrap_count (baseline: $baseline)"

if (( unwrap_count > baseline )); then
  echo "FAIL: unwrap/expect count ($unwrap_count) exceeds baseline ($baseline)" >&2
  echo ""
  echo "Offenders:"
  cat "$tmp"
  exit 1
fi

panic_baseline_file="ci/panic_prod_baseline.txt"
if [ -f "$panic_baseline_file" ]; then
  panic_baseline=$(cat "$panic_baseline_file")
else
  panic_baseline=0
fi

echo "panic-family macros: $panic_count (baseline: $panic_baseline)"

if (( panic_count > panic_baseline )); then
  echo "FAIL: panic-family count ($panic_count) exceeds baseline ($panic_baseline)" >&2
  echo ""
  echo "Offenders:"
  cat "$tmp_panic"
  echo "If you removed panic-family macros, update ci/panic_prod_baseline.txt with the new lower count."
  exit 1
fi
