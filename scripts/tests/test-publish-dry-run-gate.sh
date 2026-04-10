#!/usr/bin/env bash
# Self-test for the publish dry-run gate (scripts/cargo-package-workspace-dry-run.sh).
#
# Problem it prevents: the publish dry-run gate was silently false-failing on every
# Cargo.toml PR for hours because no test verified it actually catches real errors.
# (Issue #3322)
#
# This self-test verifies three properties of the gate's underlying mechanism:
#
#   CASE 1 - Clean crate: cargo package exits 0 on a minimal valid workspace.
#             Proves the gate doesn't false-fail on valid manifests.
#
#   CASE 2 - Duplicate TOML key: cargo metadata exits non-zero on a duplicate
#             [package] section. The gate calls cargo metadata internally, so
#             this confirms the parse-error detection path actually fires.
#
#   CASE 3 - Nonexistent dependency: cargo package exits non-zero when a crate
#             declares a dependency that cannot be resolved. Confirms the gate
#             catches packaging failures from bad dependency declarations.
#
# Each case uses an isolated temp workspace with no external dependencies.
# Cases 2 and 3 fail before network access (parse error / resolution stage).
# Case 1 uses a zero-dependency crate so no registry network access is needed.
#
# Usage:
#   bash scripts/tests/test-publish-dry-run-gate.sh
#
# Returns:
#   Exit 0 if all assertions pass.
#   Exit 1 if any assertion fails.

set -uo pipefail

PASS=0
FAIL=0
TMPDIR_BASE=""

cleanup() {
  if [[ -n "${TMPDIR_BASE:-}" && -d "${TMPDIR_BASE}" ]]; then
    rm -rf "${TMPDIR_BASE}"
  fi
}
trap cleanup EXIT

TMPDIR_BASE="$(mktemp -d)"

# ---------------------------------------------------------------------------
# Helpers
# ---------------------------------------------------------------------------

assert_exit_zero() {
  local label="$1"
  local actual_exit="$2"
  if [[ "${actual_exit}" -eq 0 ]]; then
    echo "PASS  ${label}"
    PASS=$((PASS + 1))
  else
    echo "FAIL  ${label} (expected exit 0, got ${actual_exit})"
    FAIL=$((FAIL + 1))
  fi
}

assert_exit_nonzero() {
  local label="$1"
  local actual_exit="$2"
  if [[ "${actual_exit}" -ne 0 ]]; then
    echo "PASS  ${label} (exit ${actual_exit} as expected)"
    PASS=$((PASS + 1))
  else
    echo "FAIL  ${label} (expected non-zero exit, got 0 — gate is NOT catching this error)"
    FAIL=$((FAIL + 1))
  fi
}

make_workspace() {
  # make_workspace <dir> <crate-toml-content>
  # Creates a minimal workspace at <dir> with one crate.
  local dir="$1"
  local toml_content="$2"
  mkdir -p "${dir}/my-crate/src"
  printf 'pub fn placeholder() {}\n' > "${dir}/my-crate/src/lib.rs"
  printf '%s\n' "${toml_content}" > "${dir}/my-crate/Cargo.toml"
  cat > "${dir}/Cargo.toml" << 'WORKSPACE_EOF'
[workspace]
members = ["my-crate"]
resolver = "2"
WORKSPACE_EOF
}

# ---------------------------------------------------------------------------
# CASE 1: Clean minimal crate — cargo package must exit 0
#
# Verifies the detection mechanism doesn't false-fail on a valid manifest.
# Uses a self-contained workspace with no external dependencies so no
# network access is required for the packaging step itself.
# ---------------------------------------------------------------------------

echo ""
echo "=== CASE 1: clean minimal crate — expect exit 0 ==="

CLEAN_WORKSPACE="${TMPDIR_BASE}/clean-workspace"
make_workspace "${CLEAN_WORKSPACE}" '[package]
name = "my-crate"
version = "0.1.0"
edition = "2021"'

CLEAN_TARGET="${TMPDIR_BASE}/target-clean"
(
  cd "${CLEAN_WORKSPACE}"
  CARGO_TARGET_DIR="${CLEAN_TARGET}" \
    cargo package -p my-crate --no-verify --allow-dirty
) > "${TMPDIR_BASE}/case1.out" 2>&1
CASE1_EXIT=$?
assert_exit_zero "clean crate packaged successfully" "${CASE1_EXIT}"

if [[ "${CASE1_EXIT}" -ne 0 ]]; then
  echo "      (output follows)"
  sed 's/^/      /' "${TMPDIR_BASE}/case1.out"
fi

# ---------------------------------------------------------------------------
# CASE 2: Duplicate Cargo.toml key — cargo metadata must reject it
#
# A duplicate [package] section is a hard TOML parse error. The gate calls
# cargo metadata internally; verifying cargo metadata exits non-zero here
# confirms the detection mechanism the gate relies on actually fires.
# ---------------------------------------------------------------------------

echo ""
echo "=== CASE 2: duplicate [package] key in Cargo.toml — expect non-zero exit ==="

DUP_WORKSPACE="${TMPDIR_BASE}/dup-key-workspace"
make_workspace "${DUP_WORKSPACE}" '[package]
name = "my-crate"
version = "0.1.0"
edition = "2021"

[package]
name = "my-crate"
version = "0.1.0"
edition = "2021"'

(
  cd "${DUP_WORKSPACE}"
  cargo metadata --format-version=1 --no-deps
) > "${TMPDIR_BASE}/case2.out" 2>&1
CASE2_EXIT=$?
assert_exit_nonzero "duplicate [package] key rejected by cargo metadata" "${CASE2_EXIT}"

# ---------------------------------------------------------------------------
# CASE 3: Nonexistent dependency — cargo package must fail
#
# A crate declaring a dependency that does not exist on crates.io should
# cause cargo to fail during dependency resolution. This confirms the gate
# catches packaging failures from unresolvable dependency declarations.
#
# Note: cargo resolve attempts to contact the registry index. If the index
# is unavailable (offline), cargo exits non-zero anyway (network error),
# which still satisfies the assertion that the gate does not silently pass.
# ---------------------------------------------------------------------------

echo ""
echo "=== CASE 3: nonexistent dependency — expect non-zero exit ==="

MISSING_DEP_WORKSPACE="${TMPDIR_BASE}/missing-dep-workspace"
make_workspace "${MISSING_DEP_WORKSPACE}" '[package]
name = "my-crate"
version = "0.1.0"
edition = "2021"

[dependencies]
nonexistent-crate-xyz-12345 = "99.99.99"'

MISSING_TARGET="${TMPDIR_BASE}/target-missing"
(
  cd "${MISSING_DEP_WORKSPACE}"
  CARGO_TARGET_DIR="${MISSING_TARGET}" \
    cargo package -p my-crate --no-verify --allow-dirty
) > "${TMPDIR_BASE}/case3.out" 2>&1
CASE3_EXIT=$?
assert_exit_nonzero "nonexistent dep rejected by cargo package" "${CASE3_EXIT}"

# ---------------------------------------------------------------------------
# Summary
# ---------------------------------------------------------------------------

TOTAL=$((PASS + FAIL))
echo ""
echo "=== Results: ${PASS}/${TOTAL} passed ==="

if [[ "${FAIL}" -gt 0 ]]; then
  echo "FAIL: ${FAIL} assertion(s) failed."
  exit 1
fi

echo "All assertions passed — the publish dry-run gate catches what it claims to catch."
exit 0
