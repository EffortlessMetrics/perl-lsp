#!/usr/bin/env bash
set -euo pipefail

pass() { printf '✅ %s\n' "$1"; }
warn() { printf '⚠️  %s\n' "$1"; }
fail() { printf '❌ %s\n' "$1"; }

has_cmd() {
  command -v "$1" >/dev/null 2>&1
}

check_cmd() {
  local cmd="$1"
  local label="$2"
  if has_cmd "$cmd"; then
    pass "$label: found ($(command -v "$cmd"))"
    return 0
  fi
  warn "$label: not found"
  return 1
}

check_rust_component() {
  local component="$1"
  if ! has_cmd rustup; then
    warn "rustup unavailable; cannot verify component '$component'"
    return 1
  fi

  if rustup component list --installed 2>/dev/null | awk '{print $1}' | grep -Eq "^${component}(-|$)"; then
    pass "rustup component installed: $component"
    return 0
  fi

  warn "rustup component missing: $component (install: rustup component add $component)"
  return 1
}

check_githook() {
  local hook_path=".git/hooks/pre-push"
  if [ -x "$hook_path" ]; then
    pass "pre-push git hook installed: $hook_path"
  else
    warn "pre-push git hook not installed (run: bash scripts/install-githooks.sh)"
  fi
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

parse_pinned_toolchain() {
  if [ ! -f rust-toolchain.toml ]; then
    return 1
  fi

  awk -F'"' '/channel/{print $2; exit}' rust-toolchain.toml
}

check_toolchain_alignment() {
  local pinned="$1"

  if [ -z "$pinned" ]; then
    warn "Could not parse rust-toolchain.toml"
    return 1
  fi

  pass "Pinned toolchain: $pinned"

  if ! has_cmd rustc; then
    warn "rustc unavailable; cannot verify active toolchain version"
    return 1
  fi

  local rustc_version
  rustc_version=$(rustc --version 2>/dev/null | awk '{print $2}')
  if [ -z "$rustc_version" ]; then
    warn "Could not determine active rustc version"
    return 1
  fi

  if [ "$rustc_version" = "$pinned" ]; then
    pass "Active rustc matches pinned toolchain: $rustc_version"
  else
    warn "Active rustc ($rustc_version) does not match pinned toolchain ($pinned)"
    warn "Fix with: rustup toolchain install $pinned && rustup override set $pinned"
  fi

  if has_cmd rustup; then
    local active_toolchain
    active_toolchain=$(rustup show active-toolchain 2>/dev/null | awk '{print $1}')
    if [ -n "$active_toolchain" ]; then
      pass "rustup active toolchain: $active_toolchain"
    else
      warn "Could not determine rustup active toolchain"
    fi
  fi
}

MISSING_REQUIRED=0

echo "Repository: $(pwd)"

echo
printf '== Required ==\n'
if ! check_cmd cargo "cargo"; then MISSING_REQUIRED=1; fi
if ! check_cmd rustfmt "rustfmt"; then MISSING_REQUIRED=1; fi
if ! check_cmd rustup "rustup"; then MISSING_REQUIRED=1; fi

show_version "rustc" rustc --version
show_version "cargo" cargo --version

echo
printf '== Recommended ==\n'
check_cmd just "just" || true
check_cmd nix "nix" || true
check_cmd cargo-audit "cargo-audit" || true
check_githook

echo
printf '== Rust components ==\n'
check_rust_component rustfmt || true
check_rust_component clippy || true

if PINNED_TOOLCHAIN=$(parse_pinned_toolchain 2>/dev/null); then
  check_toolchain_alignment "$PINNED_TOOLCHAIN" || true
else
  warn "rust-toolchain.toml not found"
fi

echo
printf '== Suggested next commands ==\n'
echo "  just doctor"
echo "  just pr-fast"
echo "  just devex-targeted"
echo "  just ci-gate"
echo "  nix develop -c just ci-gate"

if [ "$MISSING_REQUIRED" -ne 0 ]; then
  echo
  fail "Missing required tools. Install Rust via https://rustup.rs"
  exit 1
fi

echo
pass "Doctor completed: required tooling is available"
