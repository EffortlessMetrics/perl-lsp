#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "${SCRIPT_DIR}/../.." && pwd)"
RUN_GATES_SCRIPT="${REPO_ROOT}/scripts/run-gates.sh"

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

write_fake_cargo() {
  local fake_bin="$1"
  local log_path="$2"

  mkdir -p "$fake_bin"
  cat > "${fake_bin}/cargo" <<FAKE
#!/usr/bin/env bash
printf '%s\n' "\$@" > "${log_path}"
exit "\${FAKE_CARGO_EXIT:-0}"
FAKE
  chmod +x "${fake_bin}/cargo"
}

assert_args_equal() {
  local label="$1"
  local expected="$2"
  local actual="$3"

  if cmp -s "$expected" "$actual"; then
    pass "$label"
  else
    fail "$label"
    printf 'expected:\n'
    cat "$expected"
    printf 'actual:\n'
    cat "$actual"
  fi
}

echo "=== run-gates wrapper test suite ==="
echo ""

if [[ ! -f "$RUN_GATES_SCRIPT" ]]; then
  echo "ERROR: run-gates.sh not found at ${RUN_GATES_SCRIPT}"
  exit 1
fi

TMPDIR_BASE="$(mktemp -d)"
FAKE_BIN="${TMPDIR_BASE}/bin"
FAKE_LOG="${TMPDIR_BASE}/cargo-args.txt"
write_fake_cargo "$FAKE_BIN" "$FAKE_LOG"

DEFAULT_DIR="${TMPDIR_BASE}/default"
mkdir -p "$DEFAULT_DIR"
EXPECTED_DEFAULT_ARGS="${DEFAULT_DIR}/expected-args.txt"
{
  printf 'xtask\n'
  printf 'gates\n'
  printf '%s\n' '--tier'
  printf 'merge-gate\n'
  printf '%s\n' '--receipt'
  printf '%s\n' '--receipt-path'
  printf '%s\n' "${REPO_ROOT}/target/receipts/receipt.json"
  printf '%s\n' '--format'
  printf 'json\n'
} > "$EXPECTED_DEFAULT_ARGS"

code=0
(
  cd "$REPO_ROOT"
  PATH="${FAKE_BIN}:$PATH" bash "$RUN_GATES_SCRIPT" --format json
) > "${DEFAULT_DIR}/out.txt" 2> "${DEFAULT_DIR}/err.txt" || code=$?
assert_exit_zero "delegates default tier to cargo xtask gates" "$code"
assert_args_equal "uses merge-gate tier and forwards arguments" "$EXPECTED_DEFAULT_ARGS" "$FAKE_LOG"

FULL_DIR="${TMPDIR_BASE}/full"
mkdir -p "$FULL_DIR"
EXPECTED_FULL_ARGS="${FULL_DIR}/expected-args.txt"
{
  printf 'xtask\n'
  printf 'gates\n'
  printf '%s\n' '--tier'
  printf 'all\n'
  printf '%s\n' '--receipt'
  printf '%s\n' '--receipt-path'
  printf '%s\n' "${REPO_ROOT}/target/receipts/receipt.json"
  printf '%s\n' '--check'
} > "$EXPECTED_FULL_ARGS"

code=0
(
  cd "$REPO_ROOT"
  PATH="${FAKE_BIN}:$PATH" RUN_FULL=1 bash "$RUN_GATES_SCRIPT" --check
) > "${FULL_DIR}/out.txt" 2> "${FULL_DIR}/err.txt" || code=$?
assert_exit_zero "RUN_FULL selects all-tier gate" "$code"
assert_args_equal "uses all tier and keeps receipt output" "$EXPECTED_FULL_ARGS" "$FAKE_LOG"

FAIL_DIR="${TMPDIR_BASE}/fail"
mkdir -p "$FAIL_DIR"
code=0
(
  cd "$REPO_ROOT"
  PATH="${FAKE_BIN}:$PATH" FAKE_CARGO_EXIT=37 bash "$RUN_GATES_SCRIPT" --format json
) > "${FAIL_DIR}/out.txt" 2> "${FAIL_DIR}/err.txt" || code=$?
assert_exit_nonzero "propagates cargo failure from delegated gate tier" "$code"

TOTAL=$((PASS + FAIL))
echo ""
echo "=== Results: ${PASS}/${TOTAL} passed ==="

if [[ "$FAIL" -gt 0 ]]; then
  exit 1
fi

exit 0
