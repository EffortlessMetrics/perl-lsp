#!/usr/bin/env bash
set -euo pipefail

PASS_COUNT=0
WARN_COUNT=0
FAIL_COUNT=0
NEXT_STEPS=()

pass() {
  PASS_COUNT=$((PASS_COUNT + 1))
  printf '✅ %s\n' "$1"
}

warn() {
  WARN_COUNT=$((WARN_COUNT + 1))
  printf '⚠️  %s\n' "$1"
}

fail() {
  FAIL_COUNT=$((FAIL_COUNT + 1))
  printf '❌ %s\n' "$1"
}

rule() { printf '%s\n' "----------------------------------------------"; }

section() {
  echo
  rule
  printf '%s\n' "$1"
  rule
}

add_next_step() {
  local step="$1"
  for existing in "${NEXT_STEPS[@]:-}"; do
    if [[ "$existing" == "$step" ]]; then
      return 0
    fi
  done
  NEXT_STEPS+=("$step")
}

has_cmd() {
  command -v "$1" >/dev/null 2>&1
}

check_cmd() {
  local cmd="$1"
  local label="$2"
  if has_cmd "$cmd"; then
    pass "$label: found at $(command -v "$cmd")"
    return 0
  fi
  warn "$label: not found"
  return 1
}

check_rust_component() {
  local component="$1"
  if ! has_cmd rustup; then
    warn "rustup unavailable; cannot verify component '$component'"
    add_next_step "Install rustup so component checks can run: https://rustup.rs"
    return 1
  fi

  if rustup component list --installed 2>/dev/null | awk '{print $1}' | grep -Eq "^${component}(-|$)"; then
    pass "rustup component installed: $component"
    return 0
  fi

  warn "rustup component missing: $component"
  add_next_step "Install missing Rust component: rustup component add $component"
  return 1
}

check_githook() {
  local hook_path=".git/hooks/pre-push"
  if [ -x "$hook_path" ]; then
    pass "pre-push git hook installed: $hook_path"
  else
    warn "pre-push git hook not installed"
    add_next_step "Install the repo git hooks: bash scripts/install-githooks.sh"
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

MISSING_REQUIRED=0

echo "Repository: $(pwd)"
echo "Timestamp: $(date -u '+%Y-%m-%d %H:%M:%S UTC')"

section "Required tooling"
if ! check_cmd cargo "cargo"; then MISSING_REQUIRED=1; fi
if ! check_cmd rustfmt "rustfmt"; then
  MISSING_REQUIRED=1
  add_next_step "Install rustfmt: rustup component add rustfmt"
fi
if ! check_cmd rustup "rustup"; then MISSING_REQUIRED=1; fi

show_version "rustc" rustc --version
show_version "cargo" cargo --version

section "Recommended tooling"
check_cmd just "just" || true
check_cmd nix "nix" || true
check_cmd cargo-audit "cargo-audit" || true
check_githook

section "Rust components"
check_rust_component rustfmt || true
check_rust_component clippy || true

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

section "Developer workflow"
printf '  %-18s %s\n' "Fast PR loop" "just pr-fast"
printf '  %-18s %s\n' "Repo gate" "just ci-gate"
printf '  %-18s %s\n' "Nix gate" "nix develop -c just ci-gate"

section "Doctor summary"
printf '  %-10s %s\n' "Passes" "$PASS_COUNT"
printf '  %-10s %s\n' "Warnings" "$WARN_COUNT"
printf '  %-10s %s\n' "Failures" "$FAIL_COUNT"

if ((${#NEXT_STEPS[@]} > 0)); then
  echo
  printf 'Suggested fixes:\n'
  for step in "${NEXT_STEPS[@]}"; do
    printf '  - %s\n' "$step"
  done
else
  echo
  printf 'Suggested fixes:\n'
  printf '  - None. Your local setup looks ready for day-to-day development.\n'
fi

if [ "$MISSING_REQUIRED" -ne 0 ]; then
  echo
  fail "Missing required tools. Install Rust via https://rustup.rs"
  exit 1
fi

echo
pass "Doctor completed: required tooling is available"
