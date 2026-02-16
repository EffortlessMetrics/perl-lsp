#!/usr/bin/env bash
set -euo pipefail

# Count explicit unsafe syntax in production source code.
# This intentionally targets real language constructs, not prose in comments.
PATTERN='unsafe[[:space:]]*\{|unsafe[[:space:]]+extern|unsafe[[:space:]]+impl|#!\[allow\(unsafe_code\)\]'

# Build target list, excluding legacy/generated parser crates.
TARGETS=()
for d in crates/*/src; do
    if [[ "$d" == *"tree-sitter-perl-rs"* ]] || [[ "$d" == *"perl-parser-pest"* ]]; then
        continue
    fi
    TARGETS+=("$d")
done

tmp="$(mktemp)"
trap 'rm -f "$tmp"' EXIT

if rg -n "$PATTERN" "${TARGETS[@]}" >"$tmp"; then
  : # matches found
else
  status=$?
  if [[ "$status" -ne 1 ]]; then
    echo "rg failed (exit=$status)" >&2
    exit "$status"
  fi
  : # exit 1 = no matches, keep going
fi

count="$(wc -l <"$tmp" | tr -d ' ')"
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
