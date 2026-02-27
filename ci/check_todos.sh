#!/usr/bin/env bash
# Policy: All unlinked action items must reference a GitHub issue.
# Pattern: TODO(#123): explanation
#
# This script enforces that the count of unlinked items does not increase.
set -euo pipefail

# Configuration
BASELINE_FILE="ci/todo_baseline.txt"
EXCLUDE_DIRS=("target" ".git" ".receipts" ".runs")
# Files excluded from scanning: test fixtures with intentional markers, field docs
EXCLUDE_FILES=(
    "ci/check_todos.sh"
    "crates/perl-parser/tests/missing_docs_ac_tests.rs"
    "crates/perl-tdd-support/src/tdd/test_generator.rs"
)

# Build exclude flags for rg (applied AFTER include globs so exclusions win)
EXCLUDE_ARGS=()
for dir in "${EXCLUDE_DIRS[@]}"; do
    EXCLUDE_ARGS+=("-g" "!$dir")
done
for file in "${EXCLUDE_FILES[@]}"; do
    EXCLUDE_ARGS+=("-g" "!$file")
done

# Split-by-filetype scanning: Rust files use // and /* */ comments,
# script/config files use # comments. This prevents false positives from
# hash-comment markers inside Perl fixtures embedded in Rust raw strings.
UNLINKED_RE='(?:TODO|FIXME)(?!\s*\(#\d+\))'

# Rust: // comments (inline or start-of-line), /* block start, * block continuation
# (?:^|\s)// ensures we don't match http:// or similar URLs
RUST_RE="(?:(?<![:/\"])//[/!]?|(?:^|\\s)/\\*|^\\s*\\*\\s).*${UNLINKED_RE}"

# Scripts/config: # comments (start-of-line or inline after whitespace)
HASH_RE="(?:^|\\s)#\\s.*${UNLINKED_RE}"

# File globs for hash-comment languages
HASH_GLOBS=(-g'*.sh' -g'*.bash' -g'*.pl' -g'*.pm' -g'*.t' \
            -g'Justfile' -g'justfile' -g'*.just')

# Count unlinked items (exclude args AFTER include globs for correct precedence)
count_unlinked() {
    local rust_count hash_count
    rust_count=$(rg --type rust --pcre2 "${EXCLUDE_ARGS[@]}" "${RUST_RE}" . 2>/dev/null | wc -l)
    hash_count=$(rg --pcre2 "${HASH_GLOBS[@]}" "${EXCLUDE_ARGS[@]}" \
        "${HASH_RE}" . 2>/dev/null | wc -l)
    echo $(( rust_count + hash_count ))
}

# List unlinked items (exclude args AFTER include globs for correct precedence)
list_unlinked() {
    rg --type rust --pcre2 -n --no-heading "${EXCLUDE_ARGS[@]}" "${RUST_RE}" . 2>/dev/null || true
    rg --pcre2 -n --no-heading "${HASH_GLOBS[@]}" "${EXCLUDE_ARGS[@]}" \
        "${HASH_RE}" . 2>/dev/null || true
}

# Optional: --list mode for debugging
if [[ "${1:-}" == "--list" ]]; then
    list_unlinked
    exit 0
fi

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
