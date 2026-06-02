#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "${SCRIPT_DIR}/../.." && pwd)"
WRAPPER="${REPO_ROOT}/scripts/cleanup-worktrees.sh"

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
  local args_log="$2"
  local pwd_log="$3"

  mkdir -p "$fake_bin"
  cat > "${fake_bin}/cargo" <<FAKE
#!/usr/bin/env bash
pwd > "${pwd_log}"
printf '%s\n' "\$@" > "${args_log}"
exit "\${FAKE_CARGO_EXIT:-0}"
FAKE
  chmod +x "${fake_bin}/cargo"
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

echo "=== cleanup-worktrees wrapper test suite ==="
echo ""

if [[ ! -f "$WRAPPER" ]]; then
  echo "ERROR: cleanup-worktrees.sh not found at ${WRAPPER}"
  exit 1
fi

TMPDIR_BASE="$(mktemp -d)"
FAKE_BIN="${TMPDIR_BASE}/bin"
FAKE_ARGS_LOG="${TMPDIR_BASE}/cargo-args.txt"
FAKE_PWD_LOG="${TMPDIR_BASE}/cargo-pwd.txt"
write_fake_cargo "$FAKE_BIN" "$FAKE_ARGS_LOG" "$FAKE_PWD_LOG"

PASS_DIR="${TMPDIR_BASE}/pass"
mkdir -p "$PASS_DIR"
EXPECTED_ARGS="${PASS_DIR}/expected-args.txt"
cat > "$EXPECTED_ARGS" <<'ARGS'
xtask
worktree-cleanup
--dry-run
ARGS
EXPECTED_PWD="${PASS_DIR}/expected-pwd.txt"
printf '%s\n' "$REPO_ROOT" > "$EXPECTED_PWD"

code=0
(
  cd "${TMPDIR_BASE}"
  PATH="${FAKE_BIN}:$PATH" bash "$WRAPPER" --dry-run
) > "${PASS_DIR}/out.txt" 2> "${PASS_DIR}/err.txt" || code=$?
assert_exit_zero "delegates to cargo xtask worktree-cleanup" "$code"
assert_file_equals "runs from the wrapper repo root" "$EXPECTED_PWD" "$FAKE_PWD_LOG"
assert_file_equals "forwards worktree-cleanup arguments unchanged" "$EXPECTED_ARGS" "$FAKE_ARGS_LOG"

FAIL_DIR="${TMPDIR_BASE}/fail"
mkdir -p "$FAIL_DIR"
code=0
(
  cd "$REPO_ROOT"
  PATH="${FAKE_BIN}:$PATH" FAKE_CARGO_EXIT=37 bash "$WRAPPER" --dry-run
) > "${FAIL_DIR}/out.txt" 2> "${FAIL_DIR}/err.txt" || code=$?
assert_exit_nonzero "propagates cargo failure from delegated command" "$code"

TOTAL=$((PASS + FAIL))
echo ""
echo "=== Results: ${PASS}/${TOTAL} passed ==="

if [[ "$FAIL" -gt 0 ]]; then
  exit 1
fi

exit 0
