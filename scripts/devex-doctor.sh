#!/usr/bin/env bash
set -euo pipefail

pass() { printf '✅ %s\n' "$1"; }
warn() { printf '⚠️  %s\n' "$1"; }
fail() { printf '❌ %s\n' "$1"; }

STATUS_REQUIRED=()
STATUS_RECOMMENDED=()

record_status() {
  local bucket="$1"
  local label="$2"
  local status="$3"
  local value="$4"
  local entry="${label}|${status}|${value}"

  if [ "$bucket" = "required" ]; then
    STATUS_REQUIRED+=("$entry")
    return
  fi
  STATUS_RECOMMENDED+=("$entry")
}

render_summary() {
  local total_required=${#STATUS_REQUIRED[@]}
  local total_recommended=${#STATUS_RECOMMENDED[@]}
  local total=$((total_required + total_recommended))
  local ok=0
  local warn_count=0
  local item status

  for item in "${STATUS_REQUIRED[@]}" "${STATUS_RECOMMENDED[@]}"; do
    [ -z "$item" ] && continue
    status=${item#*|}
    status=${status%%|*}
    if [ "$status" = "ok" ]; then
      ok=$((ok + 1))
    else
      warn_count=$((warn_count + 1))
    fi
  done

  local width=20
  local filled=0
  if [ "$total" -gt 0 ]; then
    filled=$((ok * width / total))
  fi
  local bar
  bar=$(printf '%*s' "$filled" '' | tr ' ' '#')
  bar+=$(printf '%*s' "$((width - filled))" '' | tr ' ' '.')

  echo
  printf '== DevEx Health Snapshot ==\n'
  printf '  Score: [%s] %d/%d checks passing\n' "$bar" "$ok" "$total"
  printf '  Required:    %d/%d\n' "$((total_required - MISSING_REQUIRED))" "$total_required"
  printf '  Recommended: %d/%d\n' "$((total_recommended - MISSING_RECOMMENDED))" "$total_recommended"
  printf '  Needs attention: %d\n' "$warn_count"
}

render_table() {
  local title="$1"
  shift
  local entries=("$@")
  local entry label status value icon

  echo
  printf '== %s ==\n' "$title"
  printf '  %-18s | %-6s | %s\n' "Check" "Status" "Details"
  printf '  -------------------+--------+------------------------------\n'
  for entry in "${entries[@]}"; do
    [ -z "$entry" ] && continue
    label=${entry%%|*}
    status=${entry#*|}
    status=${status%%|*}
    value=${entry##*|}

    if [ "$status" = "ok" ]; then
      icon="✅"
    else
      icon="⚠️"
    fi

    printf '  %-18s | %-6s | %s\n' "$label" "$icon" "$value"
  done
}

check_cmd() {
  local cmd="$1"
  local label="$2"
  local bucket="${3:-recommended}"
  if command -v "$cmd" >/dev/null 2>&1; then
    local found
    found=$(command -v "$cmd")
    pass "$label: found ($found)"
    record_status "$bucket" "$label" "ok" "$found"
    return 0
  fi
  warn "$label: not found"
  record_status "$bucket" "$label" "warn" "not found"
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

echo "Repository: $(pwd)"

echo
printf '== Required ==\n'
if ! check_cmd cargo "cargo" required; then MISSING_REQUIRED=1; fi
if ! check_cmd rustfmt "rustfmt" required; then MISSING_REQUIRED=1; fi

show_version "rustc" rustc --version
show_version "cargo" cargo --version

echo
printf '== Recommended ==\n'
if ! check_cmd just "just" recommended; then MISSING_RECOMMENDED=$((MISSING_RECOMMENDED + 1)); fi
if ! check_cmd nix "nix" recommended; then MISSING_RECOMMENDED=$((MISSING_RECOMMENDED + 1)); fi
if ! check_cmd cargo-audit "cargo-audit" recommended; then MISSING_RECOMMENDED=$((MISSING_RECOMMENDED + 1)); fi

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

render_table "Required Tooling" "${STATUS_REQUIRED[@]}"
render_table "Recommended Tooling" "${STATUS_RECOMMENDED[@]}"
render_summary

if [ "$MISSING_REQUIRED" -ne 0 ]; then
  echo
  fail "Missing required tools. Install Rust via https://rustup.rs"
  exit 1
fi

echo
pass "Doctor completed: required tooling is available"
