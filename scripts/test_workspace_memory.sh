#!/bin/bash
# Test workspace index memory scaling at different file counts
# Usage: ./test_workspace_memory.sh [--scale N] [--json]

set -e

echo "[info] Building workspace_memory_profile binary..." >&2
cargo build --release -p perl-workspace --bin workspace_memory_profile --features memory-profiling 2>&1 | tail -5

echo "[info] Running memory profile tests..." >&2
cargo run --release -p perl-workspace --bin workspace_memory_profile --features memory-profiling -- "$@"
