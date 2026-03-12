#!/usr/bin/env bash
set -euo pipefail

PASS_COUNT=0
WARN_COUNT=0
FAIL_COUNT=0

if [ -t 1 ]; then
  BOLD='\033[1m'
  DIM='\033[2m'
  BLUE='\033[34m'
  GREEN='\033[32m'
  YELLOW='\033[33m'
  RED='\033[31m'
  RESET='\033[0m'
else
  BOLD=''
  DIM=''
  BLUE=''
  GREEN=''
  YELLOW=''
  RED=''
  RESET=''
fi

pass() {
  PASS_COUNT=$((PASS_COUNT + 1))
  printf '%b✅ %-11s%b %s\n' "$GREEN" "PASS" "$RESET" "$1"
}

warn() {
  WARN_COUNT=$((WARN_COUNT + 1))
  printf '%b⚠️  %-11s%b %s\n' "$YELLOW" "WARN" "$RESET" "$1"
}

fail() {
  FAIL_COUNT=$((FAIL_COUNT + 1))
  printf '%b❌ %-11s%b %s\n' "$RED" "FAIL" "$RESET" "$1"
}

print_section() {
  printf '\n%b%s%b\n' "$BOLD$BLUE" "$1" "$RESET"
}

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

printf '%bPerl LSP DevEx Doctor%b\n' "$BOLD" "$RESET"
printf '%bRepository:%b %s\n' "$DIM" "$RESET" "$(pwd)"

print_section "== Required =="
if ! check_cmd cargo "cargo"; then MISSING_REQUIRED=1; fi
if ! check_cmd rustfmt "rustfmt"; then MISSING_REQUIRED=1; fi

show_version "rustc" rustc --version
show_version "cargo" cargo --version

print_section "== Recommended =="
check_cmd just "just" || true
check_cmd nix "nix" || true
check_cmd cargo-audit "cargo-audit" || true

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

print_section "== Suggested next commands =="
echo "  just pr-fast"
echo "  just ci-gate"
echo "  nix develop -c just ci-gate"

print_section "== Summary =="
printf '%bPassed:%b  %d\n' "$GREEN" "$RESET" "$PASS_COUNT"
printf '%bWarnings:%b %d\n' "$YELLOW" "$RESET" "$WARN_COUNT"
printf '%bFailures:%b %d\n' "$RED" "$RESET" "$FAIL_COUNT"

if [ "$MISSING_REQUIRED" -ne 0 ]; then
  fail "Missing required tools. Install Rust via https://rustup.rs"
  exit 1
fi

pass "Doctor completed: required tooling is available"
