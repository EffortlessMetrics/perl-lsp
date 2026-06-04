#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "${SCRIPT_DIR}/../.." && pwd)"
WRAPPER="${REPO_ROOT}/scripts/dead-code-check.sh"

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

echo "=== dead-code wrapper test suite ==="
echo ""

if [[ ! -f "$WRAPPER" ]]; then
  echo "ERROR: dead-code-check.sh not found at ${WRAPPER}"
  exit 1
fi

TMPDIR_BASE="$(mktemp -d)"
FAKE_BIN="${TMPDIR_BASE}/bin"
FAKE_LOG="${TMPDIR_BASE}/cargo-args.txt"
write_fake_cargo "$FAKE_BIN" "$FAKE_LOG"

DEFAULT_DIR="${TMPDIR_BASE}/default"
mkdir -p "$DEFAULT_DIR"
EXPECTED_DEFAULT_ARGS="${DEFAULT_DIR}/expected-args.txt"
cat > "$EXPECTED_DEFAULT_ARGS" <<'ARGS'
xtask
dead-code
check
ARGS

code=0
(
  cd "$REPO_ROOT"
  PATH="${FAKE_BIN}:$PATH" bash "$WRAPPER"
) > "${DEFAULT_DIR}/out.txt" 2> "${DEFAULT_DIR}/err.txt" || code=$?
assert_exit_zero "delegates default dead-code check" "$code"
assert_args_equal "defaults to check subcommand" "$EXPECTED_DEFAULT_ARGS" "$FAKE_LOG"

STRICT_DIR="${TMPDIR_BASE}/strict"
mkdir -p "$STRICT_DIR"
EXPECTED_STRICT_ARGS="${STRICT_DIR}/expected-args.txt"
cat > "$EXPECTED_STRICT_ARGS" <<'ARGS'
xtask
dead-code
report
--format
json
--strict
ARGS

code=0
(
  cd "$REPO_ROOT"
  PATH="${FAKE_BIN}:$PATH" DEAD_CODE_STRICT=true bash "$WRAPPER" report --format json
) > "${STRICT_DIR}/out.txt" 2> "${STRICT_DIR}/err.txt" || code=$?
assert_exit_zero "delegates strict dead-code command" "$code"
assert_args_equal "appends strict mode when requested" "$EXPECTED_STRICT_ARGS" "$FAKE_LOG"

STRICT_PRESENT_DIR="${TMPDIR_BASE}/strict-present"
mkdir -p "$STRICT_PRESENT_DIR"
EXPECTED_STRICT_PRESENT_ARGS="${STRICT_PRESENT_DIR}/expected-args.txt"
cat > "$EXPECTED_STRICT_PRESENT_ARGS" <<'ARGS'
xtask
dead-code
check
--strict
ARGS

code=0
(
  cd "$REPO_ROOT"
  PATH="${FAKE_BIN}:$PATH" DEAD_CODE_STRICT=true bash "$WRAPPER" check --strict
) > "${STRICT_PRESENT_DIR}/out.txt" 2> "${STRICT_PRESENT_DIR}/err.txt" || code=$?
assert_exit_zero "delegates caller-provided strict dead-code command" "$code"
assert_args_equal "does not duplicate caller-provided strict flag" "$EXPECTED_STRICT_PRESENT_ARGS" "$FAKE_LOG"

FAIL_DIR="${TMPDIR_BASE}/fail"
mkdir -p "$FAIL_DIR"
code=0
(
  cd "$REPO_ROOT"
  PATH="${FAKE_BIN}:$PATH" FAKE_CARGO_EXIT=37 bash "$WRAPPER" check
) > "${FAIL_DIR}/out.txt" 2> "${FAIL_DIR}/err.txt" || code=$?
assert_exit_nonzero "propagates cargo failure from delegated dead-code command" "$code"

TOTAL=$((PASS + FAIL))
echo ""
echo "=== Results: ${PASS}/${TOTAL} passed ==="

if [[ "$FAIL" -gt 0 ]]; then
  exit 1
fi

exit 0
