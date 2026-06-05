#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "${SCRIPT_DIR}/../.." && pwd)"
GENERATE_SCRIPT="${REPO_ROOT}/scripts/generate-receipt.sh"

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

  if grep -Fq "$pattern" "$file"; then
    pass "$label"
  else
    fail "$label"
  fi
}

echo "=== generate-receipt test suite ==="
echo ""

if [[ ! -f "$GENERATE_SCRIPT" ]]; then
  echo "ERROR: generate-receipt.sh not found at ${GENERATE_SCRIPT}"
  exit 1
fi

TMPDIR_BASE="$(mktemp -d)"

PASS_DIR="${TMPDIR_BASE}/pass"
mkdir -p "$PASS_DIR"
printf 'ok output\n' > "${PASS_DIR}/stdout.txt"
printf 'ok stderr\n' > "${PASS_DIR}/stderr.txt"
PASS_RECEIPT="${PASS_DIR}/receipt.yaml"
code=0
(
  cd "$REPO_ROOT"
  GATE_COMMAND="cargo fmt --check --all" \
    GATE_STDOUT="${PASS_DIR}/stdout.txt" \
    GATE_STDERR="${PASS_DIR}/stderr.txt" \
    COMMIT_SHA="abc123" \
    BRANCH="proof/test" \
    EXECUTOR="local" \
    AGENT="codex" \
    PR_NUMBER="42" \
    bash "$GENERATE_SCRIPT" format 0 123 "$PASS_RECEIPT"
) > "${PASS_DIR}/out.txt" 2> "${PASS_DIR}/err.txt" || code=$?
assert_exit_zero "generates pass receipt" "$code"
assert_file_contains "pass receipt uses registry gate name" "$PASS_RECEIPT" 'gate_name: "Code Formatting"'
assert_file_contains "pass receipt records pass status" "$PASS_RECEIPT" 'status: "pass"'
assert_file_contains "pass receipt records proceed routing" "$PASS_RECEIPT" 'action: "proceed"'
assert_file_contains "pass receipt records stdout evidence" "$PASS_RECEIPT" 'ok output'
assert_file_contains "pass receipt records stderr evidence" "$PASS_RECEIPT" 'ok stderr'
assert_file_contains "pass receipt records PR number" "$PASS_RECEIPT" 'pr_number: 42'
assert_file_contains "pass receipt records executor" "$PASS_RECEIPT" 'executor: "local"'

FAIL_DIR="${TMPDIR_BASE}/fail"
mkdir -p "$FAIL_DIR"
printf 'failure details\n' > "${FAIL_DIR}/stderr.txt"
FAIL_RECEIPT="${FAIL_DIR}/receipt.yaml"
code=0
(
  cd "$REPO_ROOT"
  GATE_COMMAND="cargo test --workspace --lib" \
    GATE_STDERR="${FAIL_DIR}/stderr.txt" \
    COMMIT_SHA="def456" \
    BRANCH="proof/fail" \
    EXECUTOR="github-actions" \
    AGENT="ci" \
    bash "$GENERATE_SCRIPT" unknown-gate 7 456 "$FAIL_RECEIPT"
) > "${FAIL_DIR}/out.txt" 2> "${FAIL_DIR}/err.txt" || code=$?
assert_exit_zero "generates fail receipt" "$code"
assert_file_contains "unknown gate falls back to gate id" "$FAIL_RECEIPT" 'gate_name: "unknown-gate"'
assert_file_contains "fail receipt records fail status" "$FAIL_RECEIPT" 'status: "fail"'
assert_file_contains "fail receipt records block routing" "$FAIL_RECEIPT" 'action: "block"'
assert_file_contains "fail receipt records exit code" "$FAIL_RECEIPT" 'exit_code: 7'
assert_file_contains "fail receipt records stderr evidence" "$FAIL_RECEIPT" 'failure details'

MISSING_ARG_DIR="${TMPDIR_BASE}/missing-arg"
mkdir -p "$MISSING_ARG_DIR"
code=0
(
  cd "$REPO_ROOT"
  bash "$GENERATE_SCRIPT"
) > "${MISSING_ARG_DIR}/out.txt" 2> "${MISSING_ARG_DIR}/err.txt" || code=$?
assert_exit_nonzero "fails when gate id argument is missing" "$code"
assert_file_contains "missing argument names gate id" "${MISSING_ARG_DIR}/err.txt" "Gate ID required"

TOTAL=$((PASS + FAIL))
echo ""
echo "=== Results: ${PASS}/${TOTAL} passed ==="

if [[ "$FAIL" -gt 0 ]]; then
  exit 1
fi

exit 0
