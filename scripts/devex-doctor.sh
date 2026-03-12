#!/usr/bin/env bash
set -euo pipefail

PASS_COUNT=0
WARN_COUNT=0
FAIL_COUNT=0
DETAIL_LINES=()

pass() {
  PASS_COUNT=$((PASS_COUNT + 1))
  printf '✅ %s\n' "$1"
  DETAIL_LINES+=("✅ $1")
}

warn() {
  WARN_COUNT=$((WARN_COUNT + 1))
  printf '⚠️  %s\n' "$1"
  DETAIL_LINES+=("⚠️  $1")
}

fail() {
  FAIL_COUNT=$((FAIL_COUNT + 1))
  printf '❌ %s\n' "$1"
  DETAIL_LINES+=("❌ $1")
}

print_header() {
  local title="$1"
  printf '\n%s\n' "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
  printf ' %s\n' "$title"
  printf '%s\n' "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
}

print_metric() {
  local label="$1"
  local value="$2"
  printf '  %-18s %s\n' "$label" "$value"
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

print_header "Perl-LSP DevEx Doctor"
print_metric "Repository" "$(pwd)"

print_header "Required Tooling"
if ! check_cmd cargo "cargo"; then MISSING_REQUIRED=1; fi
if ! check_cmd rustfmt "rustfmt"; then MISSING_REQUIRED=1; fi

show_version "rustc" rustc --version
show_version "cargo" cargo --version

print_header "Recommended Tooling"
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

print_header "Suggested Next Commands"
echo "  • just pr-fast"
echo "  • just ci-gate"
echo "  • nix develop -c just ci-gate"

if [ "$MISSING_REQUIRED" -ne 0 ]; then
  print_header "Summary"
  print_metric "Passed" "$PASS_COUNT"
  print_metric "Warnings" "$WARN_COUNT"
  print_metric "Failures" "$FAIL_COUNT"
  print_metric "Overall" "❌ action required"

  fail "Missing required tools. Install Rust via https://rustup.rs"
  exit 1
fi

print_header "Summary"
print_metric "Passed" "$PASS_COUNT"
print_metric "Warnings" "$WARN_COUNT"
print_metric "Failures" "$FAIL_COUNT"
print_metric "Overall" "✅ healthy"

pass "Doctor completed: required tooling is available"
