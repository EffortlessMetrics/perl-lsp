#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "${SCRIPT_DIR}/../.." && pwd)"
WRAPPER="${REPO_ROOT}/scripts/publish-receipts.sh"

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

echo "=== publish-receipts wrapper test suite ==="
echo ""

if [[ ! -f "$WRAPPER" ]]; then
  echo "ERROR: publish-receipts.sh not found at ${WRAPPER}"
  exit 1
fi

TMPDIR_BASE="$(mktemp -d)"
FAKE_BIN="${TMPDIR_BASE}/bin"
FAKE_LOG="${TMPDIR_BASE}/cargo-args.txt"
write_fake_cargo "$FAKE_BIN" "$FAKE_LOG"

NO_ARG_DIR="${TMPDIR_BASE}/no-arg"
mkdir -p "$NO_ARG_DIR"
EXPECTED_NO_ARG="${NO_ARG_DIR}/expected-args.txt"
cat > "$EXPECTED_NO_ARG" <<'ARGS'
xtask
publish-receipts
ARGS

code=0
(
  cd "$REPO_ROOT"
  PATH="${FAKE_BIN}:$PATH" bash "$WRAPPER"
) > "${NO_ARG_DIR}/out.txt" 2> "${NO_ARG_DIR}/err.txt" || code=$?
assert_exit_zero "delegates to cargo xtask publish-receipts with no receipt path" "$code"
assert_args_equal "passes no optional receipt path by default" "$EXPECTED_NO_ARG" "$FAKE_LOG"

ONE_ARG_DIR="${TMPDIR_BASE}/one-arg"
mkdir -p "$ONE_ARG_DIR"
EXPECTED_ONE_ARG="${ONE_ARG_DIR}/expected-args.txt"
cat > "$EXPECTED_ONE_ARG" <<'ARGS'
xtask
publish-receipts
target/receipts/review-guidance.json
ARGS

code=0
(
  cd "$REPO_ROOT"
  PATH="${FAKE_BIN}:$PATH" bash "$WRAPPER" target/receipts/review-guidance.json
) > "${ONE_ARG_DIR}/out.txt" 2> "${ONE_ARG_DIR}/err.txt" || code=$?
assert_exit_zero "delegates to cargo xtask publish-receipts with one receipt path" "$code"
assert_args_equal "forwards the optional receipt path" "$EXPECTED_ONE_ARG" "$FAKE_LOG"

MULTI_ARG_DIR="${TMPDIR_BASE}/multi-arg"
mkdir -p "$MULTI_ARG_DIR"
EXPECTED_MULTI_ARG="${MULTI_ARG_DIR}/expected-args.txt"
cat > "$EXPECTED_MULTI_ARG" <<'ARGS'
xtask
publish-receipts
target/receipts/review-guidance.json
ARGS

code=0
(
  cd "$REPO_ROOT"
  PATH="${FAKE_BIN}:$PATH" bash "$WRAPPER" target/receipts/review-guidance.json ignored-extra.json
) > "${MULTI_ARG_DIR}/out.txt" 2> "${MULTI_ARG_DIR}/err.txt" || code=$?
assert_exit_zero "keeps the current first-receipt compatibility boundary" "$code"
assert_args_equal "forwards only the first optional receipt path" "$EXPECTED_MULTI_ARG" "$FAKE_LOG"

FAIL_DIR="${TMPDIR_BASE}/fail"
mkdir -p "$FAIL_DIR"
code=0
(
  cd "$REPO_ROOT"
  PATH="${FAKE_BIN}:$PATH" FAKE_CARGO_EXIT=37 bash "$WRAPPER" target/receipts/review-guidance.json
) > "${FAIL_DIR}/out.txt" 2> "${FAIL_DIR}/err.txt" || code=$?
assert_exit_nonzero "propagates cargo failure from delegated command" "$code"

TOTAL=$((PASS + FAIL))
echo ""
echo "=== Results: ${PASS}/${TOTAL} passed ==="

if [[ "$FAIL" -gt 0 ]]; then
  exit 1
fi

exit 0
