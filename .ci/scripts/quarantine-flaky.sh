#!/usr/bin/env bash
# Auto-quarantine flaky tests by adding them to the technical debt ledger.
#
# Reads a list of flaky test names and optional root-cause classification,
# then adds entries to .ci/debt-ledger.yaml under flaky_tests without
# rewriting the ledger comments or unrelated sections.
#
# Usage:
#   quarantine-flaky.sh \
#     --flake-list flaky-tests.txt \
#     --classification flake-classification.txt \
#     --ledger .ci/debt-ledger.yaml \
#     [--dry-run]
#
# Classification file format (one per line): test_name|root_cause
# Root causes: timing, race_condition, resource_leak, random_seed, network, unknown

set -euo pipefail

FLAKE_LIST=""
CLASSIFICATION=""
LEDGER=".ci/debt-ledger.yaml"
DRY_RUN=false

while [[ $# -gt 0 ]]; do
    case $1 in
        --flake-list)      FLAKE_LIST="$2"; shift 2 ;;
        --classification)  CLASSIFICATION="$2"; shift 2 ;;
        --ledger)          LEDGER="$2"; shift 2 ;;
        --dry-run)         DRY_RUN=true; shift ;;
        *) echo "Unknown option: $1"; exit 1 ;;
    esac
done

if [[ -z "$FLAKE_LIST" ]]; then
    echo "ERROR: --flake-list required"
    exit 1
fi

if [[ ! -f "$LEDGER" ]]; then
    echo "ERROR: Ledger not found: $LEDGER"
    exit 1
fi

# Load classification map
declare -A CAUSES=()
if [[ -n "$CLASSIFICATION" && -f "$CLASSIFICATION" ]]; then
    while IFS="|" read -r test cause; do
        test=${test%$'\r'}
        cause=${cause%$'\r'}
        [[ -z "$test" ]] && continue
        CAUSES["$test"]="$cause"
    done < "$CLASSIFICATION"
fi

# Check budget.
CURRENT=$(grep -cE "^  - name:" "$LEDGER" 2>/dev/null || true)
MAX=$(awk '/^[[:space:]]+max_quarantined_tests:/ {print $2; exit}' "$LEDGER")
if [[ -z "$MAX" ]]; then
    echo "ERROR: Could not read max_quarantined_tests from $LEDGER"
    exit 1
fi

ISSUE_DATE=$(date +%Y-%m-%d)
# Portable date calculation
if date --version &>/dev/null 2>&1; then
    EXPIRY_DATE=$(date -d "+7 days" +%Y-%m-%d)
else
    EXPIRY_DATE=$(date -v+7d +%Y-%m-%d)
fi

ADDED=0
SKIPPED=0

while IFS= read -r test_name; do
    test_name=${test_name%$'\r'}
    [[ -z "$test_name" ]] && continue

    # Already quarantined?
    if grep -Fq "name: \"$test_name\"" "$LEDGER" 2>/dev/null; then
        echo "SKIP: $test_name already quarantined"
        SKIPPED=$((SKIPPED + 1))
        continue
    fi

    # Budget check
    if [[ "$CURRENT" -ge "$MAX" ]]; then
        echo "ERROR: quarantine budget full ($CURRENT/$MAX)"
        exit 1
    fi

    CATEGORY="${CAUSES[$test_name]:-unknown}"

    if $DRY_RUN; then
        echo "DRY RUN: would quarantine $test_name (cause: $CATEGORY)"
        ADDED=$((ADDED + 1))
        continue
    fi

    # Update the ledger in place so the explanatory comments stay intact.
    python3 - "$LEDGER" "$test_name" "$ISSUE_DATE" "$EXPIRY_DATE" "$CATEGORY" <<'PYEOF'
import json
import sys
from pathlib import Path

ledger_file, name, added, expires, category = sys.argv[1:6]
path = Path(ledger_file)
text = path.read_text()

anchor = "flaky_tests: []"
entry = [
    '  - name: ' + json.dumps(name),
    '    added: ' + json.dumps(added),
    '    issue: null',
    '    tier: "quarantine"',
    '    quarantine_days: 7',
    '    expires: ' + json.dumps(expires),
    '    owner: null',
    '    notes: "Auto-quarantined by flake detection pipeline"',
    '    root_cause_category: ' + json.dumps(category),
    '    failure_pattern: null',
    '    affected_platforms:',
    '      - "all"',
]

if anchor in text:
    text = text.replace(anchor, "flaky_tests:\n" + "\n".join(entry), 1)
else:
    marker = "flaky_tests:"
    start = text.find(marker)
    if start == -1:
        raise SystemExit("ERROR: flaky_tests section not found")
    line_end = text.find("\n", start)
    if line_end == -1:
        line_end = len(text)
    text = text[:line_end + 1] + "\n" + "\n".join(entry) + "\n" + text[line_end + 1:]

path.write_text(text)
print(f"Quarantined: {name}")
PYEOF

    ADDED=$((ADDED + 1))
    CURRENT=$((CURRENT + 1))
done < "$FLAKE_LIST"

echo ""
echo "Quarantine summary:"
echo "  Added: $ADDED"
echo "  Skipped: $SKIPPED"
echo "  Total: $CURRENT/$MAX"
