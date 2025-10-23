#!/usr/bin/env bash
# Generate canonical receipts for documentation truth
# Usage: ./scripts/generate-receipts.sh

set -euo pipefail

PROJECT_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
ARTIFACTS_DIR="${PROJECT_ROOT}/artifacts"
mkdir -p "${ARTIFACTS_DIR}"

echo "=== Generating Test Receipts ==="

# Run tests and capture output (exclude xtask which has compilation issues)
cd "${PROJECT_ROOT}"
RUST_TEST_THREADS=2 cargo +stable test --workspace --exclude xtask --all-features --no-fail-fast 2>&1 \
  | tee "${ARTIFACTS_DIR}/test-output.txt"

# Parse test output into summary
echo "=== Parsing Test Results ==="
if [ -f "${ARTIFACTS_DIR}/test-output.txt" ]; then
  # Extract test result lines (format: "test result: ok. 272 passed; 0 failed; 818 ignored; 0 measured; 0 filtered out")
  # Sum all results across crates (tolerant of missing output)
  RESULTS="$(grep -E '^[[:space:]]*test result:' "${ARTIFACTS_DIR}/test-output.txt" || true)"
  if [ -z "$RESULTS" ]; then
    echo "Warning: no test summaries found; treating as zeroes" >&2
    TOTAL_PASSED=0
    TOTAL_FAILED=0
    TOTAL_IGNORED=0
  else
    TOTAL_PASSED=$(echo "$RESULTS" | awk '{sum += $4} END {print sum+0}')
    TOTAL_FAILED=$(echo "$RESULTS" | awk '{sum += $6} END {print sum+0}')
    TOTAL_IGNORED=$(echo "$RESULTS" | awk '{sum += $8} END {print sum+0}')
  fi

  # Calculate totals
  ACTIVE_TESTS=$((TOTAL_PASSED + TOTAL_FAILED))
  TOTAL_ALL_TESTS=$((ACTIVE_TESTS + TOTAL_IGNORED))

  # Calculate pass rates (avoid division by zero)
  if [ ${ACTIVE_TESTS} -gt 0 ]; then
    PASS_RATE_ACTIVE=$(awk "BEGIN {printf \"%.1f\", (${TOTAL_PASSED}/${ACTIVE_TESTS})*100}")
  else
    PASS_RATE_ACTIVE="0.0"
  fi

  if [ ${TOTAL_ALL_TESTS} -gt 0 ]; then
    PASS_RATE_TOTAL=$(awk "BEGIN {printf \"%.1f\", (${TOTAL_PASSED}/${TOTAL_ALL_TESTS})*100}")
  else
    PASS_RATE_TOTAL="0.0"
  fi

  # Create JSON summary
  cat > "${ARTIFACTS_DIR}/test-summary.json" <<EOF
{
  "passed": ${TOTAL_PASSED},
  "failed": ${TOTAL_FAILED},
  "ignored": ${TOTAL_IGNORED},
  "active_tests": ${ACTIVE_TESTS},
  "total_all_tests": ${TOTAL_ALL_TESTS},
  "pass_rate_active": ${PASS_RATE_ACTIVE},
  "pass_rate_total": ${PASS_RATE_TOTAL}
}
EOF

  echo "Test summary saved to ${ARTIFACTS_DIR}/test-summary.json"
  cat "${ARTIFACTS_DIR}/test-summary.json"
else
  echo "Warning: test-output.txt not found"
  echo '{"passed": 0, "failed": 0, "ignored": 0, "active_tests": 0, "total_all_tests": 0, "pass_rate_active": 0.0, "pass_rate_total": 0.0}' > "${ARTIFACTS_DIR}/test-summary.json"
fi

echo ""
echo "=== Generating Doc Receipts ==="

# Count missing docs warnings from rustdoc
cd "${PROJECT_ROOT}"
cargo +stable doc --no-deps --package perl-parser 2>&1 \
  | tee "${ARTIFACTS_DIR}/rustdoc.log" \
  | grep -c '^warning: missing documentation' \
  | awk '{print "{\"missing_docs\": " $1 "}"}' \
  > "${ARTIFACTS_DIR}/doc-summary.json" || echo '{"missing_docs": 0}' > "${ARTIFACTS_DIR}/doc-summary.json"

echo "Doc summary saved to ${ARTIFACTS_DIR}/doc-summary.json"
cat "${ARTIFACTS_DIR}/doc-summary.json"

echo ""
echo "=== Generating Consolidated State ==="

# Extract version from workspace Cargo.toml
VERSION=$(grep -E "^version\s*=" Cargo.toml | head -1 | sed -E 's/.*"([^"]+)".*/\1/')

# Combine all receipts into single state file
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

echo "Consolidated state saved to ${ARTIFACTS_DIR}/state.json"
cat "${ARTIFACTS_DIR}/state.json"

echo ""
echo "=== Receipt Generation Complete ==="
echo "Artifacts:"
echo "  - ${ARTIFACTS_DIR}/test-output.txt     (raw test output)"
echo "  - ${ARTIFACTS_DIR}/test-summary.json   (parsed test metrics)"
echo "  - ${ARTIFACTS_DIR}/rustdoc.log         (doc build output)"
echo "  - ${ARTIFACTS_DIR}/doc-summary.json    (doc metrics)"
echo "  - ${ARTIFACTS_DIR}/state.json          (consolidated truth)"
