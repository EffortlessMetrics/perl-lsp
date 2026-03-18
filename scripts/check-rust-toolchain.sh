#!/usr/bin/env bash
set -euo pipefail

MODE="${1:-check}"

pass() { printf '✅ %s\n' "$1"; }
warn() { printf '⚠️  %s\n' "$1"; }
fail() { printf '❌ %s\n' "$1"; }

if [ ! -f rust-toolchain.toml ]; then
  warn "rust-toolchain.toml not found; skipping pinned toolchain check"
  exit 0
fi

REQUIRED_TOOLCHAIN=$(awk -F'"' '/channel/{print $2; exit}' rust-toolchain.toml)
if [ -z "${REQUIRED_TOOLCHAIN:-}" ]; then
  warn "Could not parse pinned toolchain from rust-toolchain.toml"
  exit 0
fi

REQUIRED_VERSION="$REQUIRED_TOOLCHAIN"
CURRENT_VERSION=""
if command -v rustc >/dev/null 2>&1; then
  CURRENT_VERSION=$(rustc --version 2>/dev/null | awk '{print $2}')
fi

if [ -z "$CURRENT_VERSION" ]; then
  fail "rustc is not available; install Rust via https://rustup.rs"
  exit 1
fi

if [ "$CURRENT_VERSION" = "$REQUIRED_VERSION" ]; then
  pass "Rust toolchain matches pinned version: $CURRENT_VERSION"
  exit 0
fi

LOWEST=$(printf '%s\n%s\n' "$REQUIRED_VERSION" "$CURRENT_VERSION" | sort -V | head -n1)
if [ "$LOWEST" = "$REQUIRED_VERSION" ]; then
  if [ "$MODE" = "doctor" ]; then
    warn "Using Rust $CURRENT_VERSION while rust-toolchain.toml pins $REQUIRED_VERSION; builds should still work, but use 'rustup override set $REQUIRED_TOOLCHAIN' for exact parity"
  else
    pass "Rust $CURRENT_VERSION satisfies pinned MSRV $REQUIRED_VERSION"
  fi
  exit 0
fi

fail "Rust $CURRENT_VERSION is older than pinned MSRV $REQUIRED_VERSION"
printf '   Fix: rustup toolchain install %s && rustup override set %s\n' "$REQUIRED_TOOLCHAIN" "$REQUIRED_TOOLCHAIN"
exit 1
