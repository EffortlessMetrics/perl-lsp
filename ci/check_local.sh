#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
BIN="$REPO_ROOT/target/debug/perl-ci-hygiene"

if [ -x "$BIN" ]; then
  exec "$BIN" check-local "$@"
fi

exec cargo run --quiet --manifest-path "$REPO_ROOT/Cargo.toml" -p perl-ci-hygiene -- check-local "$@"
