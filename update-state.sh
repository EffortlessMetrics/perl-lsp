#!/usr/bin/env bash
set -euo pipefail

VERSION=$(grep -E "^version\s*=" Cargo.toml | head -1 | sed -E 's/.*"([^"]+)".*/\1/')
jq -n \
  --arg version "${VERSION}" \
  --slurpfile tests "artifacts/test-summary.json" \
  --slurpfile docs "artifacts/doc-summary.json" \
  '{
    version: $version,
    tests: $tests[0],
    docs: $docs[0],
    generated_at: (now | strftime("%Y-%m-%dT%H:%M:%SZ"))
  }' > artifacts/state.json

echo "Generated state.json:"
cat artifacts/state.json | jq .
