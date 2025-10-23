#!/usr/bin/env bash
# Render documentation templates with values from state.json
# Usage: ./scripts/render-docs.sh

set -euo pipefail

PROJECT_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
STATE_FILE="${PROJECT_ROOT}/artifacts/state.json"
TMP_DIR="${PROJECT_ROOT}/tmp"
DOCS_DIR="${PROJECT_ROOT}/docs"

if [ ! -f "${STATE_FILE}" ]; then
  echo "Error: State file not found at ${STATE_FILE}"
  echo "Run ./scripts/generate-receipts.sh or ./scripts/quick-receipts.sh first"
  exit 1
fi

echo "=== Rendering Documentation from Templates ==="
echo "Using state from: ${STATE_FILE}"
echo ""

# Extract values from state.json (supports both flat and nested tokens)
VERSION=$(jq -r '.version' "${STATE_FILE}")
TEST_PASSED=$(jq -r '.tests.passed // 0' "${STATE_FILE}")
TEST_FAILED=$(jq -r '.tests.failed // 0' "${STATE_FILE}")
TEST_IGNORED=$(jq -r '.tests.ignored // 0' "${STATE_FILE}")
TEST_ACTIVE=$(jq -r '.tests.active_tests // 0' "${STATE_FILE}")
TEST_TOTAL=$(jq -r '.tests.total_all_tests // 0' "${STATE_FILE}")
PASS_RATE_ACTIVE=$(jq -r '.tests.pass_rate_active // "0.0"' "${STATE_FILE}")
PASS_RATE_TOTAL=$(jq -r '.tests.pass_rate_total // "0.0"' "${STATE_FILE}")
MISSING_DOCS=$(jq -r '.docs.missing_docs // 0' "${STATE_FILE}")

echo "Loaded values:"
echo "  Version: ${VERSION}"
echo "  Tests: ${TEST_PASSED} passed, ${TEST_FAILED} failed, ${TEST_IGNORED} ignored"
echo "  Active pass rate: ${PASS_RATE_ACTIVE}%"
echo "  Total pass rate: ${PASS_RATE_TOTAL}%"
echo "  Missing docs: ${MISSING_DOCS}"
echo ""

# Function to render a file (replace tokens)
render_file() {
  local source=$1
  local target=$2

  # Use sed to replace all token variants (both flat and nested)
  sed \
    -e "s/{{version}}/${VERSION}/g" \
    -e "s/{{test_passed}}/${TEST_PASSED}/g" \
    -e "s/{{tests\.passed}}/${TEST_PASSED}/g" \
    -e "s/{{test_failed}}/${TEST_FAILED}/g" \
    -e "s/{{tests\.failed}}/${TEST_FAILED}/g" \
    -e "s/{{test_ignored}}/${TEST_IGNORED}/g" \
    -e "s/{{tests\.ignored}}/${TEST_IGNORED}/g" \
    -e "s/{{test_active}}/${TEST_ACTIVE}/g" \
    -e "s/{{tests\.active_tests}}/${TEST_ACTIVE}/g" \
    -e "s/{{test_total}}/${TEST_TOTAL}/g" \
    -e "s/{{tests\.total_all_tests}}/${TEST_TOTAL}/g" \
    -e "s/{{pass_rate_active}}/${PASS_RATE_ACTIVE}/g" \
    -e "s/{{tests\.pass_rate_active}}/${PASS_RATE_ACTIVE}/g" \
    -e "s/{{pass_rate_total}}/${PASS_RATE_TOTAL}/g" \
    -e "s/{{tests\.pass_rate_total}}/${PASS_RATE_TOTAL}/g" \
    -e "s/{{missing_docs}}/${MISSING_DOCS}/g" \
    -e "s/{{docs\.missing_docs}}/${MISSING_DOCS}/g" \
    "${source}" > "${target}"
}

# Create tmp/docs directory (mirrors docs structure)
mkdir -p "${TMP_DIR}/docs"

# Copy and render all markdown files from docs/ to tmp/docs/
echo "Rendering docs directory..."
find "${DOCS_DIR}" -type f -name "*.md" | while read -r doc_file; do
  # Calculate relative path and create target directory
  rel_path="${doc_file#${DOCS_DIR}/}"
  target_file="${TMP_DIR}/docs/${rel_path}"
  target_dir="$(dirname "${target_file}")"

  mkdir -p "${target_dir}"
  render_file "${doc_file}" "${target_file}"
  echo "  ✓ ${rel_path}"
done

# Render root markdown files (README, CLAUDE.md, etc.)
echo "Rendering root markdown files..."
for root_md in "${PROJECT_ROOT}"/*.md; do
  if [ -f "${root_md}" ]; then
    filename="$(basename "${root_md}")"
    render_file "${root_md}" "${TMP_DIR}/${filename}"
    echo "  ✓ ${filename}"
  fi
done

echo ""
echo "=== Validating Token Resolution ==="

# Check for any unresolved tokens in rendered output
if command -v rg &> /dev/null; then
  if rg -n '{{[^}]+}}' "${TMP_DIR}" 2>/dev/null | grep .; then
    echo ""
    echo "ERROR: Unresolved tokens remain in rendered docs"
    exit 1
  else
    echo "✓ All tokens resolved successfully"
  fi
else
  echo "Warning: ripgrep not found, skipping token validation"
  echo "Install ripgrep to enable validation: apt install ripgrep"
fi

echo ""
echo "=== Documentation Rendering Complete ==="
echo "Rendered docs available in: ${TMP_DIR}/"
echo "To apply changes: rsync -av ${TMP_DIR}/docs/ ${DOCS_DIR}/"
