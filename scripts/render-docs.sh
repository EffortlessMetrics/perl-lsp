#!/usr/bin/env bash
# Render documentation templates with values from state.json
# Usage: ./scripts/render-docs.sh

set -euo pipefail

PROJECT_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
STATE_FILE="${PROJECT_ROOT}/artifacts/state.json"

if [ ! -f "${STATE_FILE}" ]; then
  echo "Error: State file not found at ${STATE_FILE}"
  echo "Run ./scripts/generate-receipts.sh first"
  exit 1
fi

echo "=== Rendering Documentation from Templates ==="
echo "Using state from: ${STATE_FILE}"
echo ""

# Extract values from state.json
VERSION=$(jq -r '.version' "${STATE_FILE}")
TEST_PASSED=$(jq -r '.tests.passed' "${STATE_FILE}")
TEST_FAILED=$(jq -r '.tests.failed' "${STATE_FILE}")
TEST_IGNORED=$(jq -r '.tests.ignored' "${STATE_FILE}")
TEST_ACTIVE=$(jq -r '.tests.active_tests' "${STATE_FILE}")
TEST_TOTAL=$(jq -r '.tests.total_all_tests' "${STATE_FILE}")
PASS_RATE_ACTIVE=$(jq -r '.tests.pass_rate_active' "${STATE_FILE}")
PASS_RATE_TOTAL=$(jq -r '.tests.pass_rate_total' "${STATE_FILE}")
MISSING_DOCS=$(jq -r '.docs.missing_docs' "${STATE_FILE}")

echo "Loaded values:"
echo "  Version: ${VERSION}"
echo "  Tests: ${TEST_PASSED} passed, ${TEST_FAILED} failed, ${TEST_IGNORED} ignored"
echo "  Active pass rate: ${PASS_RATE_ACTIVE}%"
echo "  Total pass rate: ${PASS_RATE_TOTAL}%"
echo "  Missing docs: ${MISSING_DOCS}"
echo ""

# Function to render a template file
render_template() {
  local template=$1
  local output=$2

  echo "Rendering: ${template} -> ${output}"

  # Use sed to replace template tokens with values
  sed \
    -e "s/{{version}}/${VERSION}/g" \
    -e "s/{{test_passed}}/${TEST_PASSED}/g" \
    -e "s/{{test_failed}}/${TEST_FAILED}/g" \
    -e "s/{{test_ignored}}/${TEST_IGNORED}/g" \
    -e "s/{{test_active}}/${TEST_ACTIVE}/g" \
    -e "s/{{test_total}}/${TEST_TOTAL}/g" \
    -e "s/{{pass_rate_active}}/${PASS_RATE_ACTIVE}/g" \
    -e "s/{{pass_rate_total}}/${PASS_RATE_TOTAL}/g" \
    -e "s/{{missing_docs}}/${MISSING_DOCS}/g" \
    "${template}" > "${output}"
}

# Render templates
# For now, we'll render CLAUDE.md from CLAUDE.md.template if it exists
# Otherwise, we'll just show how the values would be substituted

if [ -f "${PROJECT_ROOT}/CLAUDE.md.template" ]; then
  render_template "${PROJECT_ROOT}/CLAUDE.md.template" "${PROJECT_ROOT}/CLAUDE.md"
else
  echo "Warning: CLAUDE.md.template not found"
  echo "Template tokens that should be used:"
  echo "  {{version}}           -> ${VERSION}"
  echo "  {{test_passed}}       -> ${TEST_PASSED}"
  echo "  {{test_failed}}       -> ${TEST_FAILED}"
  echo "  {{test_ignored}}      -> ${TEST_IGNORED}"
  echo "  {{test_active}}       -> ${TEST_ACTIVE}"
  echo "  {{test_total}}        -> ${TEST_TOTAL}"
  echo "  {{pass_rate_active}}  -> ${PASS_RATE_ACTIVE}%"
  echo "  {{pass_rate_total}}   -> ${PASS_RATE_TOTAL}%"
  echo "  {{missing_docs}}      -> ${MISSING_DOCS}"
fi

echo ""
echo "=== Documentation Rendering Complete ==="
