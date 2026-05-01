#!/usr/bin/env bash
set -euo pipefail

MODE="${1:-check}"
SCRIPT_DIR="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd -- "$SCRIPT_DIR/.." && pwd)"

TOOLCHAIN=""
if [ -f "$REPO_ROOT/rust-toolchain.toml" ]; then
  TOOLCHAIN=$(awk -F'"' '/channel/{print $2; exit}' "$REPO_ROOT/rust-toolchain.toml")
fi

RUSTUP_CMD=""
if command -v rustup >/dev/null 2>&1; then
  RUSTUP_CMD="$(command -v rustup)"
elif [ -n "${CARGO_HOME:-}" ] && [ -x "$CARGO_HOME/bin/rustup" ]; then
  RUSTUP_CMD="$CARGO_HOME/bin/rustup"
elif [ -n "${HOME:-}" ] && [ -x "$HOME/.cargo/bin/rustup" ]; then
  RUSTUP_CMD="$HOME/.cargo/bin/rustup"
fi

CARGO_CMD=(cargo)
if [ -n "${TOOLCHAIN:-}" ] && [ -n "$RUSTUP_CMD" ]; then
  CARGO_CMD=("$RUSTUP_CMD" run "$TOOLCHAIN" cargo)
fi

case "$MODE" in
  check)
    cd "$REPO_ROOT"
    exec "${CARGO_CMD[@]}" xtask check-toolchain
    ;;
  doctor)
    cd "$REPO_ROOT"
    exec "${CARGO_CMD[@]}" xtask check-toolchain --doctor
    ;;
  *)
    echo "Usage: $0 [check|doctor]" >&2
    exit 2
    ;;
esac
