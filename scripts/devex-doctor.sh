#!/usr/bin/env bash
set -euo pipefail

pass() { printf '✅ %s\n' "$1"; }
warn() { printf '⚠️  %s\n' "$1"; }
fail() { printf '❌ %s\n' "$1"; }

check_cmd() {
  local cmd="$1"
  local label="$2"
  if command -v "$cmd" >/dev/null 2>&1; then
    pass "$label: found ($(command -v "$cmd"))"
    return 0
  fi
  warn "$label: not found"
  return 1
}

show_version() {
  local label="$1"
  shift
  if "$@" >/dev/null 2>&1; then
    local out
    out=$("$@" 2>/dev/null | head -n1)
    pass "$label version: $out"
  else
    warn "$label version check failed"
  fi
}

MISSING_REQUIRED=0
MISSING_RECOMMENDED=0

REPO_ROOT=""
if command -v git >/dev/null 2>&1; then
  REPO_ROOT=$(git rev-parse --show-toplevel 2>/dev/null || true)
fi

echo "Repository: $(pwd)"

if [ -n "$REPO_ROOT" ]; then
  pass "Git repository root: $REPO_ROOT"
  if [ "$(pwd)" != "$REPO_ROOT" ]; then
    warn "Run doctor from repository root for best results"
  fi
else
  warn "Not inside a Git repository (or git not installed)"
fi

echo
printf '== Required ==\n'
if ! check_cmd cargo "cargo"; then MISSING_REQUIRED=1; fi
if ! check_cmd rustfmt "rustfmt"; then MISSING_REQUIRED=1; fi
if ! check_cmd rustc "rustc"; then MISSING_REQUIRED=1; fi

show_version "rustc" rustc --version
show_version "cargo" cargo --version

echo
printf '== Recommended ==\n'
check_cmd just "just" || true
check_cmd nix "nix" || true
check_cmd cargo-audit "cargo-audit" || true

if check_cmd rustup "rustup"; then
  if rustup component list --installed | rg -q '^clippy'; then
    pass "rustup component: clippy installed"
  else
    warn "rustup component: clippy missing (install with: rustup component add clippy)"
    MISSING_RECOMMENDED=1
  fi

  if rustup component list --installed | rg -q '^rustfmt'; then
    pass "rustup component: rustfmt installed"
  else
    warn "rustup component: rustfmt missing (install with: rustup component add rustfmt)"
    MISSING_REQUIRED=1
  fi
fi

if [ -n "$REPO_ROOT" ] && [ -f "$REPO_ROOT/.git/hooks/pre-push" ]; then
  pass "Git pre-push hook installed"
else
  warn "Git pre-push hook missing (install with: bash scripts/install-githooks.sh)"
  MISSING_RECOMMENDED=1
fi

if [ -f rust-toolchain.toml ]; then
  TOOLCHAIN=$(awk -F'"' '/channel/{print $2; exit}' rust-toolchain.toml)
  if [ -n "${TOOLCHAIN:-}" ]; then
    pass "Pinned toolchain: $TOOLCHAIN"
  else
    warn "Could not parse rust-toolchain.toml"
  fi
else
  warn "rust-toolchain.toml not found"
fi

echo
printf '== Suggested next commands ==\n'
echo "  just pr-fast"
echo "  just ci-gate"
echo "  nix develop -c just ci-gate"

if [ "$MISSING_REQUIRED" -ne 0 ]; then
  echo
  fail "Missing required tools. Install Rust via https://rustup.rs"
  exit 1
fi

echo
if [ "$MISSING_RECOMMENDED" -ne 0 ]; then
  warn "Doctor completed: required tools available, but some recommended setup is missing"
else
  pass "Doctor completed: required tooling is available"
fi
