#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"

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

check_git_hook() {
  local hook_path="$REPO_ROOT/.git/hooks/pre-push"
  if [ -x "$hook_path" ]; then
    pass "pre-push hook: installed ($hook_path)"
    return 0
  fi

  warn "pre-push hook: not installed"
  echo "   Install with: bash scripts/install-githooks.sh"
  return 1
}

echo "Repository: $REPO_ROOT"

if [ "$(pwd)" != "$REPO_ROOT" ]; then
  warn "Running outside repo root: $(pwd)"
  echo "   Tip: cd $REPO_ROOT"
fi

echo
printf '== Required ==\n'
if ! check_cmd cargo "cargo"; then MISSING_REQUIRED=1; fi
if ! check_cmd rustfmt "rustfmt"; then MISSING_REQUIRED=1; fi

show_version "rustc" rustc --version
show_version "cargo" cargo --version

echo
printf '== Recommended ==\n'
check_cmd just "just" || true
if ! check_cmd nix "nix"; then MISSING_RECOMMENDED=1; fi
if ! check_cmd cargo-audit "cargo-audit"; then MISSING_RECOMMENDED=1; fi

echo
printf '== Repository hygiene ==\n'
check_git_hook || true

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
if command -v just >/dev/null 2>&1; then
  echo "  just pr-fast"
else
  echo "  cargo test --workspace --lib"
fi
echo "  just ci-gate"
if command -v nix >/dev/null 2>&1; then
  echo "  nix develop -c just ci-gate"
else
  echo "  (optional) install Nix for canonical gate: https://nixos.org/download.html"
fi

if [ "$MISSING_REQUIRED" -ne 0 ]; then
  echo
  fail "Missing required tools. Install Rust via https://rustup.rs"
  exit 1
fi

echo
pass "Doctor completed: required tooling is available"

if [ "$MISSING_RECOMMENDED" -ne 0 ]; then
  warn "Doctor completed with missing recommended tools"
fi
