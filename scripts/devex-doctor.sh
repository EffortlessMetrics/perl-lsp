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

echo "Repository: $(pwd)"

if [ ! -f Cargo.toml ] || [ ! -d crates ]; then
  warn "This does not look like the repository root (expected Cargo.toml and crates/)"
  warn "Run 'just doctor' from the perl-lsp repo root for complete checks"
fi

echo
printf '== Required ==\n'
if ! check_cmd cargo "cargo"; then MISSING_REQUIRED=1; fi
if ! check_cmd rustfmt "rustfmt"; then MISSING_REQUIRED=1; fi

show_version "rustc" rustc --version
show_version "cargo" cargo --version

echo
printf '== Recommended ==\n'
check_cmd git "git" || true
check_cmd just "just" || true
check_cmd nix "nix" || true
check_cmd cargo-audit "cargo-audit" || true

if command -v rustup >/dev/null 2>&1; then
  if rustup component list --installed 2>/dev/null | grep -Eq '^rustfmt-'; then
    pass "rustup component installed: rustfmt"
  else
    warn "rustup component missing: rustfmt (install via: rustup component add rustfmt)"
  fi
else
  warn "rustup not found (recommended for toolchain/component management)"
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

if command -v git >/dev/null 2>&1 && git rev-parse --is-inside-work-tree >/dev/null 2>&1; then
  if [ -x .git/hooks/pre-push ]; then
    pass "Git pre-push hook installed (.git/hooks/pre-push)"
  else
    warn "Git pre-push hook missing (install via: bash scripts/install-githooks.sh)"
  fi
else
  warn "Not inside a Git work tree; skipping hook checks"
fi

if [ -f Cargo.lock ]; then
  NESTED_LOCKS=$(find . -name 'Cargo.lock' -type f \
    -not -path './Cargo.lock' \
    -not -path '*/target/*' \
    -not -path '*/.runs/*' \
    -not -path '*/archive/*' \
    -not -path './fuzz/*' \
    -not -path './tree-sitter-perl/*' 2>/dev/null)
  if [ -z "$NESTED_LOCKS" ]; then
    pass "No nested Cargo.lock files detected"
  else
    COUNT=$(printf '%s\n' "$NESTED_LOCKS" | sed '/^$/d' | wc -l | tr -d ' ')
    warn "Detected $COUNT nested Cargo.lock file(s); run gates from repo root"
    printf '%s\n' "$NESTED_LOCKS" | sed 's/^/  - /'
  fi
fi

echo
printf '== Suggested next commands ==\n'
echo "  just pr-fast"
echo "  just ci-gate"
if command -v nix >/dev/null 2>&1; then
  echo "  nix develop -c just ci-gate"
fi

if [ "$MISSING_REQUIRED" -ne 0 ]; then
  echo
  fail "Missing required tools. Install Rust via https://rustup.rs"
  exit 1
fi

echo
pass "Doctor completed: required tooling is available"
