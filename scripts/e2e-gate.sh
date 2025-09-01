#!/usr/bin/env bash
set -euo pipefail
# Change MAX_E2E to 3 if your box can handle it comfortably
MAX_E2E="${MAX_E2E:-2}"
LOCK="/tmp/e2e-suite.lock"

# Acquire a shared lock with a small queue (emulates -j MAX_E2E)
exec 200>"$LOCK"
# Try immediate lock; if busy, wait (keeps logs cleaner)
flock -n 200 || { echo "E2E slot busy → waiting..."; flock 200; }

# For Rust projects, run comprehensive tests with concurrency caps
RUST_TEST_THREADS="${RUST_TEST_THREADS:-2}" cargo test -- --test-threads="${RUST_TEST_THREADS:-2}" "$@"