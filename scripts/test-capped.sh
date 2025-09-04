#!/usr/bin/env bash
set -euo pipefail

# Run preflight checks and set concurrency caps
source "$(dirname "$0")/preflight.sh"

# Default capped test run for Rust
echo "Running Rust tests with ${RUST_TEST_THREADS} threads..."
RUST_TEST_THREADS="${RUST_TEST_THREADS}" cargo test -- --test-threads="${RUST_TEST_THREADS}" "$@"