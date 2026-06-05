#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "${SCRIPT_DIR}/../.." && pwd)"
EXECUTE_GATE_SCRIPT="${REPO_ROOT}/scripts/execute-gate.sh"

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

assert_file_absent() {
  local label="$1"
  local path="$2"

  if [[ ! -e "$path" ]]; then
    pass "$label"
  else
    fail "$label (${path} exists)"
  fi
}

echo "=== execute-gate wrapper test suite ==="
echo ""

if [[ ! -f "$EXECUTE_GATE_SCRIPT" ]]; then
  echo "ERROR: execute-gate.sh not found at ${EXECUTE_GATE_SCRIPT}"
  exit 1
fi

TMPDIR_BASE="$(mktemp -d)"
FAKE_BIN="${TMPDIR_BASE}/bin"
FAKE_LOG="${TMPDIR_BASE}/cargo-args.txt"
write_fake_cargo "$FAKE_BIN" "$FAKE_LOG"

MISSING_DIR="${TMPDIR_BASE}/missing"
mkdir -p "$MISSING_DIR"
code=0
(
  cd "$REPO_ROOT"
  PATH="${FAKE_BIN}:$PATH" bash "$EXECUTE_GATE_SCRIPT"
) > "${MISSING_DIR}/out.txt" 2> "${MISSING_DIR}/err.txt" || code=$?
assert_exit_nonzero "requires a gate name" "$code"
assert_file_absent "does not invoke cargo when gate name is missing" "$FAKE_LOG"

PASS_DIR="${TMPDIR_BASE}/pass"
CUSTOM_RECEIPT_DIR="${PASS_DIR}/receipts"
mkdir -p "$PASS_DIR"
EXPECTED_PASS_ARGS="${PASS_DIR}/expected-args.txt"
{
  printf 'xtask\n'
  printf 'gates\n'
  printf '%s\n' '--gate'
  printf 'policy\n'
  printf '%s\n' '--receipt'
  printf '%s\n' '--receipt-path'
  printf '%s\n' "${CUSTOM_RECEIPT_DIR}/gate-policy.json"
  printf '%s\n' '--dry-run'
  printf '%s\n' '--verbose'
} > "$EXPECTED_PASS_ARGS"

code=0
(
  cd "$REPO_ROOT"
  PATH="${FAKE_BIN}:$PATH" bash "$EXECUTE_GATE_SCRIPT" \
    policy \
    --receipt-dir "$CUSTOM_RECEIPT_DIR" \
    --dry-run \
    --verbose
) > "${PASS_DIR}/out.txt" 2> "${PASS_DIR}/err.txt" || code=$?
assert_exit_zero "delegates to cargo xtask gates with receipt output" "$code"
assert_args_equal "forwards gate arguments and custom receipt path" "$EXPECTED_PASS_ARGS" "$FAKE_LOG"

FAIL_DIR="${TMPDIR_BASE}/fail"
mkdir -p "$FAIL_DIR"
code=0
(
  cd "$REPO_ROOT"
  PATH="${FAKE_BIN}:$PATH" FAKE_CARGO_EXIT=37 bash "$EXECUTE_GATE_SCRIPT" policy
) > "${FAIL_DIR}/out.txt" 2> "${FAIL_DIR}/err.txt" || code=$?
assert_exit_nonzero "propagates cargo failure from delegated gate" "$code"

TOTAL=$((PASS + FAIL))
echo ""
echo "=== Results: ${PASS}/${TOTAL} passed ==="

if [[ "$FAIL" -gt 0 ]]; then
  exit 1
fi

exit 0
