#!/usr/bin/env bash
# Policy: All TODOs must link to a GitHub issue.
# Pattern: TODO(#123): explanation
#
# This script enforces that the number of unlinked TODOs does not increase.
set -euo pipefail

# Configuration
BASELINE_FILE="ci/todo_baseline.txt"
EXCLUDE_DIRS=("target" ".git" ".receipts" ".runs")

# Build exclude flags for rg
EXCLUDE_ARGS=()
for dir in "${EXCLUDE_DIRS[@]}"; do
    EXCLUDE_ARGS+=("-g" "!$dir")
done

# Function to count unlinked TODOs
count_unlinked() {
    # Matches TODO or FIXME NOT followed by (#number)
    # Using PCRE2 for negative lookahead
    (rg --pcre2 "${EXCLUDE_ARGS[@]}" "TODO(?!\(#\d+\))|FIXME(?!\(#\d+\))" . || true) | wc -l | xargs
}

# Function to list unlinked TODOs
list_unlinked() {
    rg --pcre2 -n --no-heading "${EXCLUDE_ARGS[@]}" "TODO(!\(#\d+\))|FIXME(!\(#\d+\))" . || true
}

# Initial baseline creation if missing
if [ ! -f "$BASELINE_FILE" ]; then
    echo "📝 Creating initial TODO baseline..."
    count_unlinked > "$BASELINE_FILE"
    echo "✅ Baseline established: $(cat "$BASELINE_FILE")"
fi

CURRENT_COUNT=$(count_unlinked)
BASELINE_COUNT=$(cat "$BASELINE_FILE")

echo "🔎 TODO Compliance Audit"
echo "======================="
echo "Current unlinked TODOs: $CURRENT_COUNT"
echo "Baseline allowed:       $BASELINE_COUNT"
echo ""

if [ "$CURRENT_COUNT" -gt "$BASELINE_COUNT" ]; then
    echo "❌ ERROR: Unlinked TODO count increased from $BASELINE_COUNT to $CURRENT_COUNT"
    echo "Please link new TODOs to a GitHub issue using the format: TODO(#123): explanation"
    echo ""
    echo "New/Unlinked violations:"
    list_unlinked
    exit 1
elif [ "$CURRENT_COUNT" -lt "$BASELINE_COUNT" ]; then
    echo "🎉 Great job! You reduced the number of unlinked TODOs ($CURRENT_COUNT < $BASELINE_COUNT)."
    echo "Please update $BASELINE_FILE to $CURRENT_COUNT to lock in this improvement."
    echo ""
    exit 0
else
    echo "✅ TODO count is within baseline limits."
    exit 0
fi
