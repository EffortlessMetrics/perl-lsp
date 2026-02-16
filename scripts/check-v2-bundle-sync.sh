#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
INCRATE_DIR="${ROOT_DIR}/crates/tree-sitter-perl-rs/src"
MICROCRATE_DIR="${ROOT_DIR}/crates/perl-parser-pest/src"

V2_BUNDLE_FILES=(
  "grammar.pest"
  "pure_rust_parser.rs"
  "pratt_parser.rs"
  "sexp_formatter.rs"
  "error.rs"
)

echo "🔍 Checking v2 bundle sync between tree-sitter-perl-rs and perl-parser-pest..."

status=0
for file in "${V2_BUNDLE_FILES[@]}"; do
  left="${INCRATE_DIR}/${file}"
  right="${MICROCRATE_DIR}/${file}"

  if ! diff -u "${left}" "${right}" > /dev/null; then
    echo "❌ Drift detected: ${file}"
    diff -u "${left}" "${right}" || true
    status=1
  else
    echo "✅ In sync: ${file}"
  fi
done

if [[ "${status}" -ne 0 ]]; then
  echo ""
  echo "v2 bundle drift detected. Synchronize the full bundle before merging."
  exit 1
fi

echo ""
echo "✅ v2 bundle is synchronized."
