#!/usr/bin/env bash
set -euo pipefail

# Count explicit unsafe syntax in production source code.
# This intentionally targets real language constructs, not prose in comments.
PATTERN='unsafe[[:space:]]*\{|unsafe[[:space:]]+extern|unsafe[[:space:]]+impl|#!\[allow\(unsafe_code\)\]'

tmp="$(mktemp)"
tmp_files="$(mktemp)"
trap 'rm -f "$tmp" "$tmp_files"' EXIT

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

count=0
>"$tmp"
while IFS= read -r file; do
  # Ignore inline test modules declared after `#[cfg(test)]`.
  test_start=$(rg -n '^\s*#\[cfg\(test\)\]' "$file" | head -1 | cut -d: -f1 || true)
  [[ -z "$test_start" ]] && test_start=999999

  matches=$(rg -nH "$PATTERN" "$file" || true)
  [[ -z "$matches" ]] && continue

  while IFS=: read -r matched_file line text; do
    if (( line < test_start )); then
      echo "$matched_file:$line:$text" >>"$tmp"
      count=$((count + 1))
    fi
  done <<<"$matches"
done <"$tmp_files"

baseline_file="ci/unsafe_prod_baseline.txt"
if [ -f "$baseline_file" ]; then
  baseline=$(cat "$baseline_file")
else
  baseline=0
fi

echo "unsafe syntax: $count (baseline: $baseline)"

if (( count > baseline )); then
  echo "FAIL: unsafe syntax count ($count) exceeds baseline ($baseline)" >&2
  echo ""
  echo "Offenders:"
  cat "$tmp"
  exit 1
fi
