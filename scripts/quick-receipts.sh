#!/usr/bin/env bash
# Quick receipt generation for version and docs (no tests)
# Usage: ./scripts/quick-receipts.sh

set -euo pipefail

PROJECT_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
ARTIFACTS_DIR="${PROJECT_ROOT}/artifacts"
mkdir -p "${ARTIFACTS_DIR}"

echo "=== Quick Receipt Generation (no tests) ==="

# Extract version from perl-parser Cargo.toml (main crate)
echo "Extracting version..."
VERSION=$(grep -E "^version\s*=" "${PROJECT_ROOT}/crates/perl-parser/Cargo.toml" | head -1 | sed -E 's/.*"([^"]+)".*/\1/')
echo "Version: ${VERSION}"

# Generate doc receipts
echo ""
echo "=== Generating Doc Receipts ==="
cd "${PROJECT_ROOT}"

# Count missing docs warnings
MISSING_DOCS=$(cargo +stable doc --no-deps --package perl-parser 2>&1 | grep -c '^warning: missing documentation' || echo "0")
echo "Missing docs: ${MISSING_DOCS}"

# Create doc summary
cat > "${ARTIFACTS_DIR}/doc-summary.json" <<EOF
{"missing_docs": ${MISSING_DOCS}}
EOF

echo "Doc summary saved to ${ARTIFACTS_DIR}/doc-summary.json"

# Create empty test summary for consistency
cat > "${ARTIFACTS_DIR}/test-summary.json" <<EOF
{
  "passed": 0,
  "failed": 0,
  "ignored": 0,
  "active_tests": 0,
  "total_all_tests": 0,
  "pass_rate_active": 0.0,
  "pass_rate_total": 0.0,
  "note": "Run generate-receipts.sh for actual test metrics"
}
EOF

# Create consolidated state.json (renderer expects this)
jq -n \
  --arg version "${VERSION}" \
  --slurpfile tests "${ARTIFACTS_DIR}/test-summary.json" \
  --slurpfile docs "${ARTIFACTS_DIR}/doc-summary.json" \
  '{
    version: $version,
    tests: $tests[0],
    docs: $docs[0],
    generated_at: (now | strftime("%Y-%m-%dT%H:%M:%SZ"))
  }' > "${ARTIFACTS_DIR}/state.json"

echo ""
echo "=== Quick Receipt Generation Complete ==="
echo "State saved to ${ARTIFACTS_DIR}/state.json (tests will be 0 until full receipt generation)"
cat "${ARTIFACTS_DIR}/state.json"
