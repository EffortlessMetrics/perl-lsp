#!/usr/bin/env bash
set -euo pipefail

PATTERN='\.unwrap\(|\.expect\('

# Build target list, excluding specific crates
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

# Filter out false positives (parser expectation method)
grep -vE "(self|s|self\.context)\.expect\(" "$tmp" > "$tmp.filtered" || true
mv "$tmp.filtered" "$tmp"

count="$(wc -l <"$tmp" | tr -d ' ')"
baseline_file="ci/unwrap_prod_baseline.txt"
if [ -f "$baseline_file" ]; then
  baseline=$(cat "$baseline_file")
else
  baseline=0
fi

echo "unwrap/expect: $count (baseline: $baseline)"

if (( count > baseline )); then
  echo "FAIL: unwrap/expect count ($count) exceeds baseline ($baseline)" >&2
  echo ""
  echo "Offenders:"
  cat "$tmp"
  exit 1
fi

# Also check for panics (ratcheting down)
PANIC_PATTERN='panic!\('
if rg -n "$PANIC_PATTERN" "${TARGETS[@]}" >"$tmp.panic"; then
  : # matches found
else
  status=$?
  if [[ "$status" -ne 1 ]]; then
    echo "rg panic check failed (exit=$status)" >&2
    exit "$status"
  fi
  : # exit 1 = no matches
fi

panic_count="$(wc -l <"$tmp.panic" | tr -d ' ')"
panic_baseline_file="ci/panic_prod_baseline.txt"
if [ -f "$panic_baseline_file" ]; then
  panic_baseline=$(cat "$panic_baseline_file")
else
  panic_baseline=0
fi

echo "panic!: $panic_count (baseline: $panic_baseline)"

if (( panic_count > panic_baseline )); then
  echo "FAIL: panic! count ($panic_count) exceeds baseline ($panic_baseline)" >&2
  echo "If you removed panics, update ci/panic_prod_baseline.txt with the new lower count."
  exit 1
fi
