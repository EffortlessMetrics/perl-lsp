#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
source "$SCRIPT_DIR/xtask_wrapper.sh"
REPORT_FILE="$REPO_ROOT/corpus_audit_report.json"

# Issue #3202: corpus-audit and check-parse-errors must run as SEPARATE
# top-level cargo invocations on Windows. Run corpus-audit first to produce
# the report file, then invoke check-parse-errors which only reads it.
ci_run_xtask_package corpus-audit --fresh --corpus-path "$REPO_ROOT" --output "$REPORT_FILE"

ci_exec_hygiene check-parse-errors "$@"
