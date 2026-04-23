#!/usr/bin/env bash
# verify-publication-facts.sh
#
# Auto-verifies all computable publication facts against expected values
# defined in docs/project/PUBLICATION_FACTS_LEDGER.md.
#
# Usage:
#   ./scripts/verify-publication-facts.sh
#   ./scripts/verify-publication-facts.sh --json      # machine-readable output
#   ./scripts/verify-publication-facts.sh --strict    # exit 1 on any ERROR (for CI)
#
# Exit codes:
#   0 — all checks passed (or only warnings)
#   1 — at least one ERROR found (threshold exceeded or mandatory check failed)
#
# Thresholds:
#   WARNING  — delta > 5%  of expected value
#   ERROR    — delta > 10% of expected value
#
# Non-automatable claims (external survey, marketplace counts, model estimates)
# are reported as informational reminders, not failures.

set -euo pipefail

REPO_ROOT="$(git rev-parse --show-toplevel)"
LEDGER="${REPO_ROOT}/docs/project/PUBLICATION_FACTS_LEDGER.md"
BASELINE_JSON="${REPO_ROOT}/.ci/cpan-corpus-baseline.json"
MANIFEST="${REPO_ROOT}/.ci/cpan-corpus-manifest.txt"
FEATURES_TOML="${REPO_ROOT}/features.toml"

# Thresholds
WARN_PCT=5
ERROR_PCT=10
# Days before a timestamp is considered stale
STALE_DAYS=30

# Output mode
JSON_MODE=false
STRICT_MODE=false
for arg in "$@"; do
    case "$arg" in
        --json)   JSON_MODE=true ;;
        --strict) STRICT_MODE=true ;;
    esac
done

# ─── Counters ─────────────────────────────────────────────────────────────────
WARNINGS=0
ERRORS=0
PASSES=0

# ─── Output helpers ───────────────────────────────────────────────────────────
if $JSON_MODE; then
    RESULTS=()
fi

_pad() { printf "%-60s" "$1"; }

emit_result() {
    local status="$1"   # PASS / WARNING / ERROR / INFO
    local metric="$2"
    local message="$3"

    case "$status" in
        PASS)    symbol="OK " ;;
        WARNING) symbol="WAR" ;;
        ERROR)   symbol="ERR" ;;
        INFO)    symbol="   " ;;
    esac

    if $JSON_MODE; then
        # Use jq --arg for JSON-safe encoding -- avoids injection from values with quotes or backslashes
        RESULTS+=("$(jq -nc --arg status "$status" --arg metric "$metric" --arg message "$message"             '{status:$status,metric:$metric,message:$message}')")
    else
        printf "%s  %s  %s\n" "$symbol" "$(_pad "$metric")" "$message"
    fi
}

# ─── Numeric delta check ──────────────────────────────────────────────────────
# Usage: check_metric <metric_name> <actual> <expected>
check_metric() {
    local name="$1"
    local actual="$2"
    local expected="$3"

    if [ "$expected" -eq 0 ]; then
        emit_result INFO "$name" "expected=0, cannot compute delta; actual=${actual}"
        return
    fi

    # Integer percent delta (bash-only, no bc required)
    local diff=$(( actual - expected ))
    if [ "$diff" -lt 0 ]; then diff=$(( -diff )); fi
    local pct=$(( diff * 100 / expected ))

    local msg="actual=${actual}  expected=${expected}  delta=${pct}%"

    if [ "$pct" -gt "$ERROR_PCT" ]; then
        emit_result ERROR "$name" "${msg} — exceeds ${ERROR_PCT}% error threshold"
        ERRORS=$(( ERRORS + 1 ))
    elif [ "$pct" -gt "$WARN_PCT" ]; then
        emit_result WARNING "$name" "${msg} — exceeds ${WARN_PCT}% warning threshold"
        WARNINGS=$(( WARNINGS + 1 ))
    else
        emit_result PASS "$name" "${msg} — OK"
        PASSES=$(( PASSES + 1 ))
    fi
}

# ─── Header ───────────────────────────────────────────────────────────────────
if ! $JSON_MODE; then
    echo "=== Publication Facts Verification ==="
    echo "Date: $(date -u +%Y-%m-%dT%H:%M:%SZ)"
    echo "Repo: ${REPO_ROOT}"
    echo ""
    echo "--- Codebase Metrics ---"
fi

# ─── 1. Lines of Rust ─────────────────────────────────────────────────────────
# Use -print0 + xargs -0 cat to avoid xargs batch-split double-counting
LOC_ACTUAL=$(find "${REPO_ROOT}/crates" -name "*.rs" -print0 | xargs -0 cat 2>/dev/null | wc -l | tr -d ' ')
LOC_EXPECTED=591034   # value from PUBLICATION_FACTS_LEDGER.md (2026-03-21)
check_metric "Lines of Rust" "$LOC_ACTUAL" "$LOC_EXPECTED"

# ─── 2. Workspace crates ──────────────────────────────────────────────────────
CRATES_ACTUAL=$(cargo metadata --no-deps --manifest-path "${REPO_ROOT}/Cargo.toml" 2>/dev/null \
    | python3 -c "import sys,json; print(len(json.load(sys.stdin)['packages']))")
CRATES_EXPECTED=133   # value from PUBLICATION_FACTS_LEDGER.md (2026-03-21)
check_metric "Workspace crates" "$CRATES_ACTUAL" "$CRATES_EXPECTED"

# ─── 3. LSP features ──────────────────────────────────────────────────────────
FEATURES_EXPECTED=98   # value from PUBLICATION_FACTS_LEDGER.md (2026-03-21)
if [ -f "$FEATURES_TOML" ]; then
    FEATURES_ACTUAL=$(grep -c '^\[\[feature\]\]' "$FEATURES_TOML" || true)
    check_metric "LSP features" "$FEATURES_ACTUAL" "$FEATURES_EXPECTED"
else
    emit_result ERROR "LSP features" "features.toml not found at ${FEATURES_TOML}"
    ERRORS=$(( ERRORS + 1 ))
fi

# ─── 4. Total commits ─────────────────────────────────────────────────────────
COMMITS_ACTUAL=$(git -C "${REPO_ROOT}" log --oneline | wc -l | tr -d ' ')
COMMITS_EXPECTED=3210   # value from PUBLICATION_FACTS_LEDGER.md (2026-03-21)
check_metric "Total commits" "$COMMITS_ACTUAL" "$COMMITS_EXPECTED"

# ─── 5. CPAN corpus total files ───────────────────────────────────────────────
if [ -f "$BASELINE_JSON" ]; then
    CORPUS_TOTAL_ACTUAL=$(python3 -c "import json; d=json.load(open('${BASELINE_JSON}')); print(d['total_files'])")
    CORPUS_CLEAN_ACTUAL=$(python3 -c "import json; d=json.load(open('${BASELINE_JSON}')); print(d['clean_files'])")

    CORPUS_TOTAL_EXPECTED=4355   # value from PUBLICATION_FACTS_LEDGER.md
    CORPUS_CLEAN_EXPECTED=3717   # baseline clean files (85.4% = 3717/4355)

    check_metric "CPAN corpus total files" "$CORPUS_TOTAL_ACTUAL" "$CORPUS_TOTAL_EXPECTED"
    check_metric "CPAN corpus clean files (baseline)" "$CORPUS_CLEAN_ACTUAL" "$CORPUS_CLEAN_EXPECTED"

    # Report baseline clean rate
    BASELINE_PCT=$(( CORPUS_CLEAN_ACTUAL * 100 / CORPUS_TOTAL_ACTUAL ))
    emit_result INFO "CPAN baseline clean rate" "${BASELINE_PCT}% (${CORPUS_CLEAN_ACTUAL}/${CORPUS_TOTAL_ACTUAL})"
else
    emit_result ERROR "CPAN corpus baseline" "${BASELINE_JSON} not found"
    ERRORS=$(( ERRORS + 1 ))
fi

# ─── 6. Corpus manifest coverage ─────────────────────────────────────────────
if [ -f "$MANIFEST" ]; then
    MANIFEST_ACTUAL=$(wc -l < "$MANIFEST" | tr -d ' ')
    MANIFEST_EXPECTED=2052   # value from PUBLICATION_FACTS_LEDGER.md (2026-03-21)
    check_metric "Corpus manifest (known-clean modules)" "$MANIFEST_ACTUAL" "$MANIFEST_EXPECTED"

    if [ "${CORPUS_TOTAL_ACTUAL:-0}" -gt 0 ]; then
        MANIFEST_PCT=$(( MANIFEST_ACTUAL * 100 / CORPUS_TOTAL_ACTUAL ))
        emit_result INFO "Corpus manifest coverage" "${MANIFEST_PCT}% (${MANIFEST_ACTUAL}/${CORPUS_TOTAL_ACTUAL:-4355})"
    fi
else
    emit_result ERROR "Corpus manifest" "${MANIFEST} not found"
    ERRORS=$(( ERRORS + 1 ))
fi

# ─── 7. Non-automatable claims (informational) ───────────────────────────────
if ! $JSON_MODE; then
    echo ""
    echo "--- Non-Automatable Claims (manual refresh required) ---"
fi

emit_result INFO "78% Perl devs use no LSP" \
    "Source: 2025 Perl IDE Survey (602 respondents). Tier D — external, unverified. Cannot auto-check."
emit_result INFO "PerlNavigator ~53K VSCode installs" \
    "Point-in-time from VSCode Marketplace. Date-stamp unknown. Refresh manually from marketplace."
emit_result INFO "Perl::LanguageServer ~293K VSCode installs" \
    "Point-in-time from VSCode Marketplace. Date-stamp unknown. Refresh manually from marketplace."
emit_result INFO "DevLT 3-5 min/PR" \
    "Model estimate from COST_ROI_ANALYSIS.md. Not measured from CI receipts. See Section 5 methodology."
emit_result INFO "Cost: \$40-79K vs \$500K-1.2M" \
    "Model estimate from COST_ROI_ANALYSIS.md Section 9. Confidence intervals documented there."

# ─── 8. Staleness check on ledger ────────────────────────────────────────────
if ! $JSON_MODE; then
    echo ""
    echo "--- Ledger Staleness ---"
fi

if [ -f "$LEDGER" ]; then
    # Extract the most recent date from the ledger (YYYY-MM-DD format)
    LEDGER_DATE=$(grep -oE '[0-9]{4}-[0-9]{2}-[0-9]{2}' "$LEDGER" | sort -r | head -1 || true)
    if [ -n "$LEDGER_DATE" ]; then
        TODAY=$(date -u +%Y-%m-%d)
        TODAY_TS=$(date -u -d "$TODAY" +%s 2>/dev/null || date -u -j -f "%Y-%m-%d" "$TODAY" +%s 2>/dev/null || echo 0)
        LEDGER_TS=$(date -u -d "$LEDGER_DATE" +%s 2>/dev/null || date -u -j -f "%Y-%m-%d" "$LEDGER_DATE" +%s 2>/dev/null || echo 0)

        if [ "$TODAY_TS" -gt 0 ] && [ "$LEDGER_TS" -gt 0 ]; then
            AGE_DAYS=$(( (TODAY_TS - LEDGER_TS) / 86400 ))
            if [ "$AGE_DAYS" -gt "$STALE_DAYS" ]; then
                emit_result WARNING "PUBLICATION_FACTS_LEDGER.md freshness" \
                    "Last entry: ${LEDGER_DATE} (${AGE_DAYS} days ago). Consider refreshing metrics."
                WARNINGS=$(( WARNINGS + 1 ))
            else
                emit_result PASS "PUBLICATION_FACTS_LEDGER.md freshness" \
                    "Last entry: ${LEDGER_DATE} (${AGE_DAYS} days ago) — within ${STALE_DAYS}-day window"
                PASSES=$(( PASSES + 1 ))
            fi
        else
            emit_result INFO "PUBLICATION_FACTS_LEDGER.md freshness" \
                "Last date found: ${LEDGER_DATE} — could not parse as timestamp on this platform"
        fi
    else
        emit_result INFO "PUBLICATION_FACTS_LEDGER.md freshness" \
            "No date entries found in ledger"
    fi
else
    emit_result ERROR "PUBLICATION_FACTS_LEDGER.md" "${LEDGER} not found"
    ERRORS=$(( ERRORS + 1 ))
fi

# ─── Summary ──────────────────────────────────────────────────────────────────
if $JSON_MODE; then
    printf '{"date":"%s","passes":%d,"warnings":%d,"errors":%d,"results":[%s]}\n' \
        "$(date -u +%Y-%m-%dT%H:%M:%SZ)" \
        "$PASSES" "$WARNINGS" "$ERRORS" \
        "$(IFS=','; echo "${RESULTS[*]}")"
else
    echo ""
    echo "=== Summary ==="
    echo "PASS:    ${PASSES}"
    echo "WARNING: ${WARNINGS}"
    echo "ERROR:   ${ERRORS}"
    echo ""
    if [ "$ERRORS" -gt 0 ]; then
        echo "RESULT: FAIL — ${ERRORS} error(s) require attention before publication."
    elif [ "$WARNINGS" -gt 0 ]; then
        echo "RESULT: WARN — ${WARNINGS} warning(s). Review before publication."
    else
        echo "RESULT: PASS — All computable metrics within thresholds."
    fi
fi

# ─── Exit code ────────────────────────────────────────────────────────────────
if $STRICT_MODE && [ "$ERRORS" -gt 0 ]; then
    exit 1
fi
# Non-strict mode: exit 0 even with warnings/errors so it can be used informally
exit 0
