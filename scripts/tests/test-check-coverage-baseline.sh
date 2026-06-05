#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "${SCRIPT_DIR}/../.." && pwd)"
CHECK_SCRIPT="${REPO_ROOT}/scripts/check-coverage-baseline.sh"

PASS=0
FAIL=0
TMPDIR_BASE=""

cleanup() {
  if [[ -n "${TMPDIR_BASE:-}" && -d "${TMPDIR_BASE}" ]]; then
    rm -rf "${TMPDIR_BASE}"
  fi
}
trap cleanup EXIT

pass() {
  printf 'PASS %s\n' "$1"
  PASS=$((PASS + 1))
}

fail() {
  printf 'FAIL %s\n' "$1"
  FAIL=$((FAIL + 1))
}

assert_exit_zero() {
  local label="$1"
  local code="$2"
  if [[ "$code" -eq 0 ]]; then
    pass "$label"
  else
    fail "$label (expected exit 0, got ${code})"
  fi
}

assert_exit_nonzero() {
  local label="$1"
  local code="$2"
  if [[ "$code" -ne 0 ]]; then
    pass "$label (exit ${code} as expected)"
  else
    fail "$label (expected non-zero exit, got 0)"
  fi
}

write_lcov() {
  local path="$1"
  local branch_hit="$2"
  local branch_found="$3"
  local line_hit="$4"
  local line_found="$5"

  cat > "$path" <<LCOV
TN:
SF:src/lib.rs
BRF:${branch_found}
BRH:${branch_hit}
LF:${line_found}
LH:${line_hit}
end_of_record
LCOV
}

write_baseline() {
  local path="$1"
  cat > "$path" <<'BASELINE'
baseline_branch_coverage=100.00 # comments are allowed
baseline_line_coverage=90.00
allowed_drop_percentage=5.00
target_branch_coverage=95.00
BASELINE
}

echo "=== check-coverage-baseline test suite ==="
echo ""

if [[ ! -f "$CHECK_SCRIPT" ]]; then
  echo "ERROR: check-coverage-baseline.sh not found at ${CHECK_SCRIPT}"
  exit 1
fi

TMPDIR_BASE="$(mktemp -d)"

PASS_DIR="${TMPDIR_BASE}/pass"
mkdir -p "$PASS_DIR"
write_lcov "${PASS_DIR}/lcov.info" 4 4 9 10
write_baseline "${PASS_DIR}/baseline.txt"
PASS_SUMMARY="${PASS_DIR}/summary.md"
PASS_OUTPUT="${PASS_DIR}/out.txt"
PASS_ERROR="${PASS_DIR}/err.txt"
code=0
bash "$CHECK_SCRIPT" "${PASS_DIR}/lcov.info" "${PASS_DIR}/baseline.txt" "$PASS_SUMMARY" \
  > "$PASS_OUTPUT" 2> "$PASS_ERROR" || code=$?
assert_exit_zero "passes when branch coverage stays within budget" "$code"
if grep -q "Gate status | pass" "$PASS_SUMMARY"; then
  pass "writes passing markdown summary"
else
  fail "writes passing markdown summary"
fi

FAIL_DIR="${TMPDIR_BASE}/fail"
mkdir -p "$FAIL_DIR"
write_lcov "${FAIL_DIR}/lcov.info" 3 4 9 10
write_baseline "${FAIL_DIR}/baseline.txt"
code=0
bash "$CHECK_SCRIPT" "${FAIL_DIR}/lcov.info" "${FAIL_DIR}/baseline.txt" \
  > "${FAIL_DIR}/out.txt" 2> "${FAIL_DIR}/err.txt" || code=$?
assert_exit_nonzero "fails when branch coverage drops past budget" "$code"
if grep -q "branch coverage dropped" "${FAIL_DIR}/err.txt"; then
  pass "failure explains branch coverage drop"
else
  fail "failure explains branch coverage drop"
fi

MISSING_LCOV_DIR="${TMPDIR_BASE}/missing-lcov"
mkdir -p "$MISSING_LCOV_DIR"
write_baseline "${MISSING_LCOV_DIR}/baseline.txt"
code=0
bash "$CHECK_SCRIPT" "${MISSING_LCOV_DIR}/missing.info" "${MISSING_LCOV_DIR}/baseline.txt" \
  > "${MISSING_LCOV_DIR}/out.txt" 2> "${MISSING_LCOV_DIR}/err.txt" || code=$?
assert_exit_nonzero "fails when lcov file is missing" "$code"
if grep -q "coverage file not found" "${MISSING_LCOV_DIR}/err.txt"; then
  pass "missing lcov failure names missing coverage file"
else
  fail "missing lcov failure names missing coverage file"
fi

MISSING_KEY_DIR="${TMPDIR_BASE}/missing-key"
mkdir -p "$MISSING_KEY_DIR"
write_lcov "${MISSING_KEY_DIR}/lcov.info" 4 4 9 10
cat > "${MISSING_KEY_DIR}/baseline.txt" <<'BASELINE'
baseline_branch_coverage=100.00
allowed_drop_percentage=5.00
BASELINE
code=0
bash "$CHECK_SCRIPT" "${MISSING_KEY_DIR}/lcov.info" "${MISSING_KEY_DIR}/baseline.txt" \
  > "${MISSING_KEY_DIR}/out.txt" 2> "${MISSING_KEY_DIR}/err.txt" || code=$?
assert_exit_nonzero "fails when required baseline keys are missing" "$code"
if grep -q "baseline file is missing required keys" "${MISSING_KEY_DIR}/err.txt"; then
  pass "missing key failure names baseline key problem"
else
  fail "missing key failure names baseline key problem"
fi

TOTAL=$((PASS + FAIL))
echo ""
echo "=== Results: ${PASS}/${TOTAL} passed ==="

if [[ "$FAIL" -gt 0 ]]; then
  exit 1
fi

exit 0
