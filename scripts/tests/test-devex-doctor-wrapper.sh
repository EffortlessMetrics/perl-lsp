#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "${SCRIPT_DIR}/../.." && pwd)"
WRAPPER_SOURCE="${REPO_ROOT}/scripts/devex-doctor.sh"

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

write_fake_cargo_safe() {
  local script_dir="$1"
  local args_log="$2"
  local pwd_log="$3"

  mkdir -p "$script_dir"
  cat > "${script_dir}/cargo-safe" <<FAKE
#!/usr/bin/env bash
pwd > "${pwd_log}"
printf '%s\n' "\$@" > "${args_log}"
exit "\${FAKE_CARGO_SAFE_EXIT:-0}"
FAKE
  chmod +x "${script_dir}/cargo-safe"
}

assert_file_equals() {
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

echo "=== devex-doctor wrapper test suite ==="
echo ""

if [[ ! -f "$WRAPPER_SOURCE" ]]; then
  echo "ERROR: devex-doctor.sh not found at ${WRAPPER_SOURCE}"
  exit 1
fi

TMPDIR_BASE="$(mktemp -d)"
TEST_REPO="${TMPDIR_BASE}/repo"
TEST_SCRIPTS="${TEST_REPO}/scripts"
mkdir -p "$TEST_SCRIPTS"
cp "$WRAPPER_SOURCE" "${TEST_SCRIPTS}/devex-doctor.sh"

FAKE_ARGS_LOG="${TMPDIR_BASE}/cargo-safe-args.txt"
FAKE_PWD_LOG="${TMPDIR_BASE}/cargo-safe-pwd.txt"
write_fake_cargo_safe "$TEST_SCRIPTS" "$FAKE_ARGS_LOG" "$FAKE_PWD_LOG"

PASS_DIR="${TMPDIR_BASE}/pass"
mkdir -p "$PASS_DIR"
EXPECTED_ARGS="${PASS_DIR}/expected-args.txt"
cat > "$EXPECTED_ARGS" <<'ARGS'
xtask
devex-doctor
ARGS
EXPECTED_PWD="${PASS_DIR}/expected-pwd.txt"
printf '%s\n' "$TEST_REPO" > "$EXPECTED_PWD"

code=0
(
  cd "$REPO_ROOT"
  bash "${TEST_SCRIPTS}/devex-doctor.sh"
) > "${PASS_DIR}/out.txt" 2> "${PASS_DIR}/err.txt" || code=$?
assert_exit_zero "delegates to scripts/cargo-safe xtask devex-doctor" "$code"
assert_file_equals "runs from the wrapper repo root" "$EXPECTED_PWD" "$FAKE_PWD_LOG"
assert_file_equals "passes only the devex-doctor xtask command" "$EXPECTED_ARGS" "$FAKE_ARGS_LOG"

FAIL_DIR="${TMPDIR_BASE}/fail"
mkdir -p "$FAIL_DIR"
code=0
(
  cd "$REPO_ROOT"
  FAKE_CARGO_SAFE_EXIT=37 bash "${TEST_SCRIPTS}/devex-doctor.sh"
) > "${FAIL_DIR}/out.txt" 2> "${FAIL_DIR}/err.txt" || code=$?
assert_exit_nonzero "propagates cargo-safe failure from delegated command" "$code"

TOTAL=$((PASS + FAIL))
echo ""
echo "=== Results: ${PASS}/${TOTAL} passed ==="

if [[ "$FAIL" -gt 0 ]]; then
  exit 1
fi

exit 0
