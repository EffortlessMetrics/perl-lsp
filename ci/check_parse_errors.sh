#!/bin/bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
BIN="$REPO_ROOT/target/debug/perl-ci-hygiene"
REPORT_FILE="$REPO_ROOT/corpus_audit_report.json"

# Issue #3202: corpus-audit and check-parse-errors must run as SEPARATE
# top-level cargo invocations on Windows. Run corpus-audit first to produce
# the report file, then invoke check-parse-errors which only reads it.
cargo run --quiet --manifest-path "$REPO_ROOT/Cargo.toml" -p xtask --no-default-features -- \
  corpus-audit --fresh --corpus-path "$REPO_ROOT" --output "$REPORT_FILE"

if [ -x "$BIN" ]; then
  exec "$BIN" check-parse-errors "$@"
fi

exec cargo run --quiet --manifest-path "$REPO_ROOT/Cargo.toml" -p perl-ci-hygiene -- check-parse-errors "$@"
