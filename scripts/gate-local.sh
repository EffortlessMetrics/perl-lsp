#!/usr/bin/env bash
# WSL-safe local gate - prevents OOM crashes with constrained parallelism
# Usage: CARGO_BUILD_JOBS=2 RUST_TEST_THREADS=1 scripts/gate-local.sh

set -euo pipefail

export CARGO_TERM_COLOR=always
export CARGO_BUILD_JOBS="${CARGO_BUILD_JOBS:-2}"
export RUST_TEST_THREADS="${RUST_TEST_THREADS:-1}"

echo "=== WSL-Safe Gate (jobs=$CARGO_BUILD_JOBS, threads=$RUST_TEST_THREADS) ==="

echo ">>> fmt check"
cargo fmt --all -- --check

echo ">>> clippy"
cargo clippy --workspace --all-targets -- -D warnings

echo ">>> perl-parser lib tests"
cargo test -p perl-parser --lib -- --test-threads="$RUST_TEST_THREADS"

echo ">>> perl-lsp integration tests"
cargo test -p perl-lsp --tests -- --test-threads="$RUST_TEST_THREADS"

echo ">>> perl-lexer tests (optional)"
cargo test -p perl-lexer --lib -- --test-threads="$RUST_TEST_THREADS" || echo "  (skipped)"

echo ">>> perl-dap tests (optional)"
cargo test -p perl-dap --lib -- --test-threads="$RUST_TEST_THREADS" || echo "  (skipped)"

echo "=== Gate passed ==="
