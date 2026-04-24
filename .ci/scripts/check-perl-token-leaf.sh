#!/usr/bin/env bash
set -euo pipefail

cargo tree -p perl-token --edges normal --prefix none --charset ascii > /tmp/perl-token-tree.txt

# First line is the root package. Any additional non-empty lines indicate runtime deps.
runtime_dep_count="$(tail -n +2 /tmp/perl-token-tree.txt | sed '/^\s*$/d' | wc -l | tr -d ' ')"
if [[ "$runtime_dep_count" != "0" ]]; then
  echo "::error::perl-token must remain std-only at runtime (found ${runtime_dep_count} runtime dependency entries)" >&2
  tail -n +2 /tmp/perl-token-tree.txt >&2
  exit 1
fi

# Manifest-level ratchet: [dependencies] should remain empty except comments/whitespace.
manifest_dep_lines="$(awk '
  /^\[dependencies\]/{in_dep=1; next}
  /^\[/{if(in_dep){exit}}
  in_dep {print}
' crates/perl-token/Cargo.toml | sed 's/#.*$//' | sed '/^\s*$/d' | wc -l | tr -d ' ')"
if [[ "$manifest_dep_lines" != "0" ]]; then
  echo "::error::crates/perl-token/Cargo.toml [dependencies] must remain empty" >&2
  exit 1
fi

echo "✅ perl-token leaf dependency guard passed"
