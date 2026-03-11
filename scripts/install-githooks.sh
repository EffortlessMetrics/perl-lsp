#!/usr/bin/env bash
set -euo pipefail
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
BIN="$REPO_ROOT/target/debug/perl-ci-hygiene"

if [ -x "$BIN" ]; then
  echo "Using existing perl-ci-hygiene binary: $BIN"
  exec "$BIN" install-githooks
fi

echo "Building perl-ci-hygiene (first run may take a minute)..."
exec cargo run --manifest-path "$REPO_ROOT/Cargo.toml" -p perl-ci-hygiene -- install-githooks
