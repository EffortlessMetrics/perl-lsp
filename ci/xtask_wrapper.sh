#!/usr/bin/env bash
set -euo pipefail

ci_repo_root() {
  local script_dir
  script_dir="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
  cd -- "$script_dir/.." && pwd
}

ci_resolve_cargo() {
  local repo_root="$1"
  local toolchain=""
  local rustup_cmd=""

  if [ -f "$repo_root/rust-toolchain.toml" ]; then
    toolchain=$(awk -F'"' '/channel/{print $2; exit}' "$repo_root/rust-toolchain.toml")
  fi

  if command -v rustup >/dev/null 2>&1; then
    rustup_cmd="$(command -v rustup)"
  elif [ -n "${CARGO_HOME:-}" ] && [ -x "$CARGO_HOME/bin/rustup" ]; then
    rustup_cmd="$CARGO_HOME/bin/rustup"
  elif [ -n "${HOME:-}" ] && [ -x "$HOME/.cargo/bin/rustup" ]; then
    rustup_cmd="$HOME/.cargo/bin/rustup"
  fi

  CI_CARGO_CMD=(cargo)
  if [ -n "${toolchain:-}" ] && [ -n "$rustup_cmd" ]; then
    CI_CARGO_CMD=("$rustup_cmd" run "$toolchain" cargo)
  fi
}

ci_run_xtask_package() {
  local repo_root
  repo_root="$(ci_repo_root)"
  ci_resolve_cargo "$repo_root"
  cd "$repo_root"
  "${CI_CARGO_CMD[@]}" run --quiet --manifest-path "$repo_root/Cargo.toml" -p xtask --no-default-features -- "$@"
}

ci_exec_xtask() {
  local repo_root
  repo_root="$(ci_repo_root)"
  ci_resolve_cargo "$repo_root"
  cd "$repo_root"
  exec "${CI_CARGO_CMD[@]}" xtask "$@"
}

ci_exec_hygiene() {
  local command="$1"
  shift
  ci_exec_xtask ci-hygiene "$command" "$@"
}
