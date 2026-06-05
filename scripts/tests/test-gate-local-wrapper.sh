#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "${SCRIPT_DIR}/../.." && pwd)"
GATE_LOCAL_SCRIPT="${REPO_ROOT}/scripts/gate-local.sh"

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
rendered=""
for arg in "\$@"; do
  rendered+="[\${arg}]"
done
printf 'env CARGO_BUILD_JOBS=%s RUST_TEST_THREADS=%s GATE_RELEASE=%s args=%s\n' \
  "\${CARGO_BUILD_JOBS:-}" \
  "\${RUST_TEST_THREADS:-}" \
  "\${GATE_RELEASE:-}" \
  "\${rendered}" >> "${log_path}"
if [[ -n "\${FAKE_CARGO_FAIL_PATTERN:-}" && "\${rendered}" == *"\${FAKE_CARGO_FAIL_PATTERN}"* ]]; then
  exit "\${FAKE_CARGO_EXIT:-37}"
fi
exit 0
FAKE
  chmod +x "${fake_bin}/cargo"
}

assert_log_has_line() {
  local label="$1"
  local log_path="$2"
  local expected="$3"

  if grep -Fqx "$expected" "$log_path"; then
    pass "$label"
  else
    fail "$label"
    printf 'missing expected line:\n%s\n' "$expected"
    printf 'actual log:\n'
    cat "$log_path"
  fi
}

assert_log_lacks_line() {
  local label="$1"
  local log_path="$2"
  local unexpected="$3"

  if grep -Fqx "$unexpected" "$log_path"; then
    fail "$label"
    printf 'unexpected line:\n%s\n' "$unexpected"
    printf 'actual log:\n'
    cat "$log_path"
  else
    pass "$label"
  fi
}

echo "=== gate-local wrapper test suite ==="
echo ""

if [[ ! -f "$GATE_LOCAL_SCRIPT" ]]; then
  echo "ERROR: gate-local.sh not found at ${GATE_LOCAL_SCRIPT}"
  exit 1
fi

TMPDIR_BASE="$(mktemp -d)"
FAKE_BIN="${TMPDIR_BASE}/bin"
DEFAULT_LOG="${TMPDIR_BASE}/default-cargo.txt"
write_fake_cargo "$FAKE_BIN" "$DEFAULT_LOG"

DEFAULT_DIR="${TMPDIR_BASE}/default"
mkdir -p "$DEFAULT_DIR"
code=0
(
  cd "$REPO_ROOT"
  PATH="${FAKE_BIN}:$PATH" bash "$GATE_LOCAL_SCRIPT"
) > "${DEFAULT_DIR}/out.txt" 2> "${DEFAULT_DIR}/err.txt" || code=$?
assert_exit_zero "delegates default local gate through cargo" "$code"
assert_log_has_line \
  "runs ignored-test-count with default caps" \
  "$DEFAULT_LOG" \
  "env CARGO_BUILD_JOBS=2 RUST_TEST_THREADS=1 GATE_RELEASE= args=[xtask][ci-hygiene][ignored-test-count]"
assert_log_has_line \
  "runs fmt check through cargo xtask" \
  "$DEFAULT_LOG" \
  "env CARGO_BUILD_JOBS=2 RUST_TEST_THREADS=1 GATE_RELEASE= args=[xtask][fmt][--check]"
assert_log_has_line \
  "runs clippy with strict warnings" \
  "$DEFAULT_LOG" \
  "env CARGO_BUILD_JOBS=2 RUST_TEST_THREADS=1 GATE_RELEASE= args=[clippy][--workspace][--all-targets][--][-D][warnings]"
assert_log_has_line \
  "runs minimal feature check" \
  "$DEFAULT_LOG" \
  "env CARGO_BUILD_JOBS=2 RUST_TEST_THREADS=1 GATE_RELEASE= args=[check][-p][perl-parser][--no-default-features]"
assert_log_has_line \
  "builds perl-lsp in debug profile by default" \
  "$DEFAULT_LOG" \
  "env CARGO_BUILD_JOBS=2 RUST_TEST_THREADS=1 GATE_RELEASE= args=[build][-p][perl-lsp-rs]"
assert_log_lacks_line \
  "does not pass release flag in default mode" \
  "$DEFAULT_LOG" \
  "env CARGO_BUILD_JOBS=2 RUST_TEST_THREADS=1 GATE_RELEASE= args=[build][-p][perl-lsp-rs][--release]"
assert_log_has_line \
  "passes default rust test thread cap" \
  "$DEFAULT_LOG" \
  "env CARGO_BUILD_JOBS=2 RUST_TEST_THREADS=1 GATE_RELEASE= args=[test][-p][perl-parser][--lib][--][--test-threads=1]"

RELEASE_LOG="${TMPDIR_BASE}/release-cargo.txt"
write_fake_cargo "$FAKE_BIN" "$RELEASE_LOG"
RELEASE_DIR="${TMPDIR_BASE}/release"
mkdir -p "$RELEASE_DIR"
code=0
(
  cd "$REPO_ROOT"
  PATH="${FAKE_BIN}:$PATH" GATE_RELEASE=1 CARGO_BUILD_JOBS=4 RUST_TEST_THREADS=3 \
    bash "$GATE_LOCAL_SCRIPT"
) > "${RELEASE_DIR}/out.txt" 2> "${RELEASE_DIR}/err.txt" || code=$?
assert_exit_zero "delegates release local gate through cargo" "$code"
assert_log_has_line \
  "keeps caller-provided cargo job cap" \
  "$RELEASE_LOG" \
  "env CARGO_BUILD_JOBS=4 RUST_TEST_THREADS=3 GATE_RELEASE=1 args=[xtask][fmt][--check]"
assert_log_has_line \
  "passes release flag to binary build" \
  "$RELEASE_LOG" \
  "env CARGO_BUILD_JOBS=4 RUST_TEST_THREADS=3 GATE_RELEASE=1 args=[build][-p][perl-lsp-rs][--release]"
assert_log_has_line \
  "passes release flag and caller thread cap to binary version test" \
  "$RELEASE_LOG" \
  "env CARGO_BUILD_JOBS=4 RUST_TEST_THREADS=3 GATE_RELEASE=1 args=[test][-p][perl-lsp-rs][--test][binary_version_test][--release][--][--test-threads=3]"

FAIL_LOG="${TMPDIR_BASE}/fail-cargo.txt"
write_fake_cargo "$FAKE_BIN" "$FAIL_LOG"
FAIL_DIR="${TMPDIR_BASE}/fail"
mkdir -p "$FAIL_DIR"
code=0
(
  cd "$REPO_ROOT"
  PATH="${FAKE_BIN}:$PATH" FAKE_CARGO_FAIL_PATTERN="[clippy]" FAKE_CARGO_EXIT=37 \
    bash "$GATE_LOCAL_SCRIPT"
) > "${FAIL_DIR}/out.txt" 2> "${FAIL_DIR}/err.txt" || code=$?
assert_exit_nonzero "propagates cargo failure from local gate" "$code"

TOTAL=$((PASS + FAIL))
echo ""
echo "=== Results: ${PASS}/${TOTAL} passed ==="

if [[ "$FAIL" -gt 0 ]]; then
  exit 1
fi

exit 0
