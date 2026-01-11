#!/usr/bin/env bash
# CI script to verify PARSER_FEATURE_MATRIX.md is in sync with corpus audit.
# Issue #180: Parser matrix drift detection.
#
# This is a drift lock: documentation must reflect the latest audit output.
#
# Exit codes:
#   0 = in sync
#   1 = drift detected
#   2 = precondition failure (missing file, etc.)

set -euo pipefail

MATRIX_FILE="docs/PARSER_FEATURE_MATRIX.md"
REPORT_FILE="corpus_audit_report.json"

# --- Precondition checks ---
if [[ ! -f "$MATRIX_FILE" ]]; then
    echo "Matrix file not found: $MATRIX_FILE"
    echo "Run: just parser-audit && just parser-matrix-update"
    exit 2
fi

# --- Generate fresh report if needed ---
# Reuse existing corpus_audit_report.json if present (common after ci-parser-features-check)
if [[ ! -f "$REPORT_FILE" ]]; then
    echo "Generating corpus audit report..."
    cargo run -p xtask --no-default-features -q -- corpus-audit --fresh --corpus-path . --output "$REPORT_FILE" 2>/dev/null || true
fi

if [[ ! -f "$REPORT_FILE" ]]; then
    echo "Report file not generated: $REPORT_FILE"
    exit 2
fi

# --- Temp file with trap cleanup ---
tmp_dir="$(mktemp -d)"
tmp_matrix="$tmp_dir/PARSER_FEATURE_MATRIX.md"
trap 'rm -rf "$tmp_dir"' EXIT

# --- Generate fresh matrix to temp file ---
python3 scripts/update-parser-matrix.py --report "$REPORT_FILE" --output "$tmp_matrix" --quiet

# --- Normalize volatile rows ---
# Remove timestamp and commit SHA rows which change on every generation
normalize_matrix() {
    sed -E \
        -e 's/^\| Generated \|.*\|$/| Generated | (elided) |/' \
        -e 's/^\| Commit \|.*\|$/| Commit | (elided) |/' \
        "$1"
}

current_normalized=$(normalize_matrix "$MATRIX_FILE")
fresh_normalized=$(normalize_matrix "$tmp_matrix")

# --- Compare ---
if [[ "$current_normalized" == "$fresh_normalized" ]]; then
    echo "Parser matrix is in sync"
    exit 0
fi

# --- Show diff on failure ---
echo ""
echo "DRIFT DETECTED: docs/PARSER_FEATURE_MATRIX.md is out of date"
echo ""
echo "Diff (--- current / +++ expected):"
echo "─────────────────────────────────"
diff -u <(echo "$current_normalized") <(echo "$fresh_normalized") || true
echo "─────────────────────────────────"
echo ""
echo "To fix:"
echo "  1. Run: just parser-audit"
echo "  2. Run: just parser-matrix-update"
echo "  3. Commit the updated docs/PARSER_FEATURE_MATRIX.md"
exit 1
