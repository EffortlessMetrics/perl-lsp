#!/bin/bash

set -euo pipefail

# SCRIPT_DIR is <repo>/benchmarks/scripts, so REPO_ROOT needs two levels up
# (benchmarks/scripts/../.. -> <repo>), not one (which previously resolved to
# <repo>/benchmarks and made every path below it, including the cargo
# manifest, point one directory too deep — see #3979).
SCRIPT_DIR="$(CDPATH= cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(CDPATH= cd -- "${SCRIPT_DIR}/../.." && pwd)"
BIN="${REPO_ROOT}/target/debug/perl-ci-hygiene"

if [ -x "$BIN" ]; then
  exec "$BIN" compare-benchmarks "$@"
fi

exec cargo run --quiet --manifest-path "$REPO_ROOT/Cargo.toml" -p perl-ci-hygiene -- compare-benchmarks "$@"
