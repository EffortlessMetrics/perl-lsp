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

# Create partial state (will be completed when tests finish)
cat > "${ARTIFACTS_DIR}/state-partial.json" <<EOF
{
  "version": "${VERSION}",
  "docs": {
    "missing_docs": ${MISSING_DOCS}
  },
  "generated_at": "$(date -u +%Y-%m-%dT%H:%M:%SZ)"
}
EOF

echo ""
echo "=== Quick Receipt Generation Complete ==="
echo "Partial state saved to ${ARTIFACTS_DIR}/state-partial.json"
cat "${ARTIFACTS_DIR}/state-partial.json"
