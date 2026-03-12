#!/usr/bin/env bash
set -euo pipefail

STRICT_MODE=0
for arg in "$@"; do
  case "$arg" in
    --strict)
      STRICT_MODE=1
      ;;
    -h|--help)
      cat <<'EOF'
Usage: scripts/devex-doctor.sh [--strict]

Options:
  --strict  Fail if recommended tools are missing.
  -h, --help  Show this help message.
EOF
      exit 0
      ;;
    *)
      printf '❌ Unknown argument: %s\n' "$arg"
      exit 2
      ;;
  esac
done

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

check_rustfmt_component() {
  if ! command -v rustup >/dev/null 2>&1; then
    warn "rustup not found; cannot verify rustfmt component"
    return 0
  fi

  if rustup component list --installed | rg -q '^rustfmt'; then
    pass "rustfmt component installed"
  else
    warn "rustfmt component missing (run: rustup component add rustfmt)"
    return 1
  fi
}

check_git_hook() {
  if [ -x ".git/hooks/pre-push" ]; then
    pass "Git pre-push hook installed"
  else
    warn "Git pre-push hook not installed (run: bash scripts/install-githooks.sh)"
    return 1
  fi
}

MISSING_REQUIRED=0
MISSING_RECOMMENDED=0

echo "Repository: $(pwd)"

echo
printf '== Required ==\n'
if ! check_cmd cargo "cargo"; then MISSING_REQUIRED=1; fi
if ! check_cmd rustfmt "rustfmt"; then MISSING_REQUIRED=1; fi
if ! check_rustfmt_component; then MISSING_REQUIRED=1; fi

show_version "rustc" rustc --version
show_version "cargo" cargo --version

echo
printf '== Recommended ==\n'
if ! check_cmd just "just"; then MISSING_RECOMMENDED=1; fi
if ! check_cmd nix "nix"; then MISSING_RECOMMENDED=1; fi
if ! check_cmd cargo-audit "cargo-audit"; then MISSING_RECOMMENDED=1; fi
if ! check_git_hook; then MISSING_RECOMMENDED=1; fi

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

if [ "$STRICT_MODE" -eq 1 ] && [ "$MISSING_RECOMMENDED" -ne 0 ]; then
  echo
  fail "Strict mode enabled and recommended tools are missing"
  exit 1
fi

echo
pass "Doctor completed: required tooling is available"
