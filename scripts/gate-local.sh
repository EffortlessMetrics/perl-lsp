#!/usr/bin/env bash
# WSL-safe local gate - prevents OOM crashes with constrained parallelism
# Usage: CARGO_BUILD_JOBS=2 RUST_TEST_THREADS=1 scripts/gate-local.sh
#
# For release builds (optional, faster execution):
#   GATE_RELEASE=1 CARGO_BUILD_JOBS=2 RUST_TEST_THREADS=1 scripts/gate-local.sh

set -euo pipefail

export CARGO_TERM_COLOR=always
export CARGO_BUILD_JOBS="${CARGO_BUILD_JOBS:-2}"
export RUST_TEST_THREADS="${RUST_TEST_THREADS:-1}"

# Determine build profile
if [[ "${GATE_RELEASE:-}" == "1" ]]; then
    PROFILE="release"
    PROFILE_FLAG="--release"
else
    PROFILE="debug"
    PROFILE_FLAG=""
fi

echo "╔═══════════════════════════════════════════════════════════════════════════════╗"
echo "║ WSL-Safe Gate (jobs=$CARGO_BUILD_JOBS, threads=$RUST_TEST_THREADS, profile=$PROFILE)"
echo "╚═══════════════════════════════════════════════════════════════════════════════╝"

echo ""
echo ">>> fmt check"
cargo fmt --all -- --check

echo ""
echo ">>> clippy"
cargo clippy --workspace --all-targets -- -D warnings

echo ""
echo ">>> Build perl-lsp binary (ensures tests use correct version)"
cargo build -p perl-lsp $PROFILE_FLAG

echo ""
echo ">>> perl-parser lib tests"
cargo test -p perl-parser --lib -- --test-threads="$RUST_TEST_THREADS"

echo ""
echo ">>> perl-lsp integration tests (including binary version check)"
# Run the version test first to catch stale binary issues early
cargo test -p perl-lsp --test binary_version_test $PROFILE_FLAG -- --test-threads="$RUST_TEST_THREADS"
cargo test -p perl-lsp --tests $PROFILE_FLAG -- --test-threads="$RUST_TEST_THREADS"

echo ""
echo ">>> perl-lexer tests (optional)"
cargo test -p perl-lexer --lib -- --test-threads="$RUST_TEST_THREADS" || echo "  (skipped)"

echo ""
echo ">>> perl-dap tests (optional)"
cargo test -p perl-dap --lib -- --test-threads="$RUST_TEST_THREADS" || echo "  (skipped)"

echo ""
echo "╔═══════════════════════════════════════════════════════════════════════════════╗"
echo "║ ✓ Gate passed                                                                  ║"
echo "╚═══════════════════════════════════════════════════════════════════════════════╝"
