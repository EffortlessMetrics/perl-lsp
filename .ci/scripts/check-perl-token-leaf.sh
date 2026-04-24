#!/usr/bin/env bash
set -Euo pipefail

cd "$(git rev-parse --show-toplevel)"

runtime_tree="$(cargo tree -p perl-token --edges normal --prefix none)"
line_count="$(printf '%s\n' "$runtime_tree" | wc -l | tr -d ' ')"
if [[ "$line_count" -ne 1 ]]; then
  echo "::error::perl-token must remain a leaf crate with no runtime dependencies"
  printf '%s\n' "$runtime_tree"
  exit 1
fi

cargo test -p perl-token --test api_contract_guards --locked

echo "✅ perl-token leaf and API contract checks passed"
