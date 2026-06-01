#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "${SCRIPT_DIR}/../.." && pwd)"
UPDATE_SCRIPT="${REPO_ROOT}/scripts/update-coverage-baseline.sh"

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

assert_file_contains() {
  local label="$1"
  local file="$2"
  local pattern="$3"

  if grep -q "$pattern" "$file"; then
    pass "$label"
  else
    fail "$label"
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

echo "=== update-coverage-baseline test suite ==="
echo ""

if [[ ! -f "$UPDATE_SCRIPT" ]]; then
  echo "ERROR: update-coverage-baseline.sh not found at ${UPDATE_SCRIPT}"
  exit 1
fi

TMPDIR_BASE="$(mktemp -d)"

DEFAULT_DIR="${TMPDIR_BASE}/default"
mkdir -p "$DEFAULT_DIR"
write_lcov "${DEFAULT_DIR}/lcov.info" 3 4 7 10
DEFAULT_BASELINE="${DEFAULT_DIR}/nested/baseline.txt"
code=0
bash "$UPDATE_SCRIPT" "${DEFAULT_DIR}/lcov.info" "$DEFAULT_BASELINE" \
  > "${DEFAULT_DIR}/out.txt" 2> "${DEFAULT_DIR}/err.txt" || code=$?
assert_exit_zero "creates baseline file from lcov" "$code"
assert_file_contains "creates parent directory for baseline" "${DEFAULT_DIR}/out.txt" "Updated ${DEFAULT_BASELINE}"
assert_file_contains "writes default coverage scope" "$DEFAULT_BASELINE" "coverage_scope=perl-parser-lib"
assert_file_contains "writes branch coverage percentage" "$DEFAULT_BASELINE" "baseline_branch_coverage=75.00"
assert_file_contains "writes line coverage percentage" "$DEFAULT_BASELINE" "baseline_line_coverage=70.00"
assert_file_contains "writes default allowed drop" "$DEFAULT_BASELINE" "allowed_drop_percentage=1.00"
assert_file_contains "writes default target branch coverage" "$DEFAULT_BASELINE" "target_branch_coverage=80.00"

PRESERVE_DIR="${TMPDIR_BASE}/preserve"
mkdir -p "$PRESERVE_DIR"
write_lcov "${PRESERVE_DIR}/lcov.info" 2 2 5 5
PRESERVE_BASELINE="${PRESERVE_DIR}/baseline.txt"
cat > "$PRESERVE_BASELINE" <<'BASELINE'
schema_version=1
coverage_scope=workspace-quality # preserve comments by stripping them
baseline_branch_coverage=10.00
baseline_line_coverage=20.00
allowed_drop_percentage=2.50
target_branch_coverage=90.00
BASELINE
code=0
bash "$UPDATE_SCRIPT" "${PRESERVE_DIR}/lcov.info" "$PRESERVE_BASELINE" \
  > "${PRESERVE_DIR}/out.txt" 2> "${PRESERVE_DIR}/err.txt" || code=$?
assert_exit_zero "updates existing baseline" "$code"
assert_file_contains "preserves existing coverage scope" "$PRESERVE_BASELINE" "coverage_scope=workspace-quality"
assert_file_contains "preserves existing allowed drop" "$PRESERVE_BASELINE" "allowed_drop_percentage=2.50"
assert_file_contains "preserves existing target coverage" "$PRESERVE_BASELINE" "target_branch_coverage=90.00"
assert_file_contains "updates existing branch coverage" "$PRESERVE_BASELINE" "baseline_branch_coverage=100.00"
assert_file_contains "updates existing line coverage" "$PRESERVE_BASELINE" "baseline_line_coverage=100.00"

MISSING_DIR="${TMPDIR_BASE}/missing"
mkdir -p "$MISSING_DIR"
code=0
bash "$UPDATE_SCRIPT" "${MISSING_DIR}/missing.info" "${MISSING_DIR}/baseline.txt" \
  > "${MISSING_DIR}/out.txt" 2> "${MISSING_DIR}/err.txt" || code=$?
assert_exit_nonzero "fails when lcov file is missing" "$code"
assert_file_contains "missing lcov failure names missing file" "${MISSING_DIR}/err.txt" "coverage file not found"

TOTAL=$((PASS + FAIL))
echo ""
echo "=== Results: ${PASS}/${TOTAL} passed ==="

if [[ "$FAIL" -gt 0 ]]; then
  exit 1
fi

exit 0
