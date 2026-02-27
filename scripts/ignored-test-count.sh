#!/bin/bash
# Counts ignored tests by category and tracks delta
#
# Usage:
#   ./scripts/ignored-test-count.sh           # Show current counts
#   ./scripts/ignored-test-count.sh --update  # Update baseline
#   ./scripts/ignored-test-count.sh --check   # CI gate (exit 1 if increased)
#
# Categories tracked:
#   - brokenpipe: BrokenPipe/transport flakes
#   - feature: Feature-gated/not implemented
#   - infra: Infrastructure/pending items
#   - protocol: Protocol compliance issues
#   - manual: Manual helper tests (snapshot regen, etc.)
#   - stress: Stress tests (run with --features stress-tests)
#   - bug: Known bugs waiting to be fixed
#   - bare: No reason given
#   - other: Everything else

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
BASELINE_FILE="$SCRIPT_DIR/.ignored-baseline"

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Counters
declare -A counts
declare -A baseline
counts[brokenpipe]=0
counts[feature]=0
counts[infra]=0
counts[protocol]=0
counts[manual]=0
counts[stress]=0
counts[bug]=0
counts[bare]=0
counts[other]=0

# Parse command line args
MODE="show"
if [[ "${1:-}" == "--update" ]]; then
    MODE="update"
elif [[ "${1:-}" == "--check" ]]; then
    MODE="check"
elif [[ "${1:-}" == "--help" || "${1:-}" == "-h" ]]; then
    echo "Usage: $0 [--update|--check|--help]"
    echo ""
    echo "Options:"
    echo "  --update  Update the baseline file with current counts"
    echo "  --check   CI gate mode: exit 1 if total ignores increased"
    echo "  --help    Show this help message"
    exit 0
fi

# Function to categorize an ignore based on reason text
categorize_ignore() {
    local reason="$1"
    # shellcheck disable=SC2034  # context reserved for future use
    local context="$2"

    # Convert to lowercase for matching
    local lower_reason="${reason,,}"

    # IMPORTANT: Check explicit prefixes FIRST before pattern matching.
    # Order matters! Explicit labels override implicit pattern matching.

    # Check for manual helper tests (snapshot regen, etc.) - MUST be first
    if [[ "$lower_reason" =~ ^manual:|manual\ |regenerate|helper ]]; then
        echo "manual"
    # Check for stress tests (should be run with --ignored for stress testing)
    elif [[ "$lower_reason" =~ ^stress:|stress\ test|memory.stress|performance.stress|load.test|stack.overflow|designed.to.fail ]]; then
        echo "stress"
    # Check for known bugs that need fixing (explicit BUG: prefix FIRST)
    elif [[ "$lower_reason" =~ ^bug:|bug:\ |known.bug|regression|incorrect.behavior|parser.bug|missing.*notification|missing.*initialize|server.returns.*instead|exposes.*|will.kill|mut_[0-9]+|known.inconsistencies|matching.issue|investigate|instead.of.expected|different.error.format|expects.*but.implementation ]]; then
        echo "bug"
    # Check for infrastructure/pending (explicit prefix BEFORE brokenpipe patterns)
    # Also match inline pending markers like "edge case - needs fix later"
    elif [[ "$lower_reason" =~ ^todo:|^infra:|infra\ |todo:|fixme|needs|requires|setup|config|environment|run.with|only.run.after|only.run.when ]]; then
        echo "infra"
    # Check for feature-gated/not implemented (before brokenpipe)
    elif [[ "$lower_reason" =~ ^feature:|feature\ |not.implemented|unimplemented|wip|work.in.progress|pending|when.implemented|remove.when|ac[0-9]+:|not.yet|tdd.scaffold|scaffold|doesn\'t.support|doesn.t.support|doesn.t.handle|parser.limitation|expected.to.fail|not.fully.supported|enable.after|after.phase|parser.doesn ]]; then
        echo "feature"
    # Check for BrokenPipe/transport issues (explicit BROKENPIPE: prefix or patterns)
    # NOTE: Generic "timeout|race" patterns removed - use explicit BROKENPIPE: label instead
    elif [[ "$lower_reason" =~ ^brokenpipe:|brokenpipe\ |broken.pipe|transport.error|transport.flake ]]; then
        echo "brokenpipe"
    # Check for protocol compliance
    elif [[ "$lower_reason" =~ protocol|lsp|dap|compliance|spec|specification ]]; then
        echo "protocol"
    # Tracked issues (feature work tracked in GitHub issues)
    elif [[ "$lower_reason" =~ tracked.in.\# ]]; then
        echo "feature"
    # AST/parser missing fields/nodes (feature gaps)
    elif [[ "$lower_reason" =~ doesn.t.have.*field|may.not.produce|doesn.t.yet|fewer.*than.expected ]]; then
        echo "feature"
    # Explicit flaky prefix
    elif [[ "$lower_reason" =~ ^flaky: ]]; then
        echo "brokenpipe"
    # Known limitations
    elif [[ "$lower_reason" =~ known.limitation ]]; then
        echo "feature"
    # Recursion/behavior changes (semantic changes)
    elif [[ "$lower_reason" =~ recursion.limit.behavior|behavior.changed ]]; then
        echo "feature"
    # Integration tests that spawn external processes
    elif [[ "$lower_reason" =~ integration.test.that.spawns|spawns.external ]]; then
        echo "infra"
    # Warnings burn-down
    elif [[ "$lower_reason" =~ warnings.burn|burn.down|clippy.warnings ]]; then
        echo "infra"
    # Mutation hardening (parser output format changes)
    elif [[ "$lower_reason" =~ mutation.hardening|parser.output.format ]]; then
        echo "feature"
    # Acceptance criteria scaffolds (AC:N patterns without other context)
    elif [[ "$lower_reason" =~ ^ac:[0-9]+ ]]; then
        echo "feature"
    # Bare ignore (no reason)
    elif [[ -z "$reason" || "$reason" == "ignore" ]]; then
        echo "bare"
    else
        echo "other"
    fi
}

# Function to extract reason from ignore attribute
extract_reason() {
    local line="$1"

    # Match #[ignore = "reason"] or #[ignore = 'reason']
    if [[ "$line" =~ \#\[ignore[[:space:]]*=[[:space:]]*\"([^\"]+)\" ]]; then
        echo "${BASH_REMATCH[1]}"
    elif [[ "$line" =~ \#\[ignore[[:space:]]*=[[:space:]]*\'([^\']+)\' ]]; then
        echo "${BASH_REMATCH[1]}"
    # Match #[ignore] with comment on same or next line
    elif [[ "$line" =~ //[[:space:]]*(.+)$ ]]; then
        echo "${BASH_REMATCH[1]}"
    else
        echo ""
    fi
}

# Find all Rust test files and scan for ignores
echo -e "${BLUE}Scanning for ignored tests in $REPO_ROOT/crates...${NC}"
echo ""

# Temporary file for detailed output
DETAILS_FILE=$(mktemp)
trap 'rm -f "$DETAILS_FILE"' EXIT

# Find all #[ignore] attributes in Rust files
while IFS= read -r file; do
    # Get relative path for cleaner output
    rel_path="${file#"$REPO_ROOT"/}"

    # Use grep to find ignore lines with context
    while IFS= read -r match; do
        if [[ -z "$match" ]]; then
            continue
        fi

        # Extract line number and content
        line_num=$(echo "$match" | cut -d: -f1)
        line_content=$(echo "$match" | cut -d: -f2-)

        # Get surrounding context (next 3 lines for function name)
        context=$(sed -n "$((line_num)),$(( line_num + 3 ))p" "$file" 2>/dev/null || echo "")

        # Extract the reason
        reason=$(extract_reason "$line_content")

        # If no reason in the ignore line, check the context
        if [[ -z "$reason" ]]; then
            # Look for comment in context
            if [[ "$context" =~ //[[:space:]]*(.+) ]]; then
                reason="${BASH_REMATCH[1]}"
            fi
        fi

        # Categorize
        category=$(categorize_ignore "$reason" "$context")
        ((counts[$category]++)) || true

        # Extract test function name if possible
        test_name=""
        if [[ "$context" =~ fn[[:space:]]+([a-zA-Z_][a-zA-Z0-9_]*) ]]; then
            test_name="${BASH_REMATCH[1]}"
        fi

        # Record details
        echo "$category|$rel_path:$line_num|$test_name|$reason" >> "$DETAILS_FILE"

    # Note: Only match #[ignore at start of line (with optional leading whitespace)
    # This eliminates false positives from commented-out ignores
    done < <(grep -nE '^[[:space:]]*#\[ignore' "$file" 2>/dev/null || true)

done < <(find "$REPO_ROOT/crates" -name "*.rs" -type f 2>/dev/null)

# Calculate total
total=0
for cat in brokenpipe feature infra protocol manual stress bug bare other; do
    ((total += counts[$cat])) || true
done

# Load baseline if exists
baseline_total=0
if [[ -f "$BASELINE_FILE" ]]; then
    while IFS='=' read -r key value; do
        # Skip comment lines (starting with #) and empty lines
        [[ "$key" =~ ^[[:space:]]*# ]] && continue
        [[ -z "$key" ]] && continue
        if [[ -n "$value" ]]; then
            baseline[$key]=$value
            if [[ "$key" == "total" ]]; then
                baseline_total=$value
            fi
        fi
    done < "$BASELINE_FILE"
fi

# Function to format delta
format_delta() {
    local current=$1
    local base=${2:-0}
    local delta=$((current - base))

    if [[ $delta -gt 0 ]]; then
        echo -e "${RED}+$delta${NC}"
    elif [[ $delta -lt 0 ]]; then
        echo -e "${GREEN}$delta${NC}"
    else
        echo "0"
    fi
}

# Print summary table
echo "==============================================="
echo "        Ignored Tests Summary"
echo "==============================================="
printf "%-12s %8s %8s %8s\n" "Category" "Count" "Baseline" "Delta"
echo "-----------------------------------------------"

for cat in brokenpipe feature infra protocol manual stress bug bare other; do
    base_val=${baseline[$cat]:-0}
    delta=$(format_delta "${counts[$cat]}" "$base_val")
    printf "%-12s %8d %8d %8b\n" "$cat" "${counts[$cat]}" "$base_val" "$delta"
done

echo "-----------------------------------------------"
base_total=${baseline[total]:-0}
delta=$(format_delta "$total" "$base_total")
printf "%-12s %8d %8d %8b\n" "TOTAL" "$total" "$base_total" "$delta"
echo "==============================================="

# Two-number split: CI_DEBT vs BACKLOG vs PERMANENT
ci_debt=$((${counts[brokenpipe]} + ${counts[bug]} + ${counts[bare]} + ${counts[other]}))
backlog=$((${counts[feature]} + ${counts[infra]}))
permanent=$((${counts[manual]} + ${counts[stress]}))
echo ""
printf "CI_DEBT    = %-3d  (brokenpipe + bug + bare + other; must be 0)\n" "$ci_debt"
printf "BACKLOG    = %-3d  (feature + infra; planned work)\n" "$backlog"
printf "PERMANENT  = %-3d  (manual + stress; bench/helpers)\n" "$permanent"
echo ""

# Show details by category if verbose
if [[ "${VERBOSE:-}" == "1" ]]; then
    echo ""
    echo "Detailed breakdown by category:"
    echo ""
    for cat in brokenpipe feature infra protocol manual stress bug bare other; do
        if [[ ${counts[$cat]} -gt 0 ]]; then
            echo -e "${YELLOW}=== $cat (${counts[$cat]}) ===${NC}"
            grep "^$cat|" "$DETAILS_FILE" | while IFS='|' read -r _ loc name reason; do
                printf "  %s\n" "$loc"
                if [[ -n "$name" ]]; then
                    printf "    fn: %s\n" "$name"
                fi
                if [[ -n "$reason" ]]; then
                    printf "    reason: %s\n" "$reason"
                fi
            done
            echo ""
        fi
    done
fi

# Handle modes
case "$MODE" in
    update)
        echo "Updating baseline file: $BASELINE_FILE"
        {
            # Use portable ISO 8601 date format (works on macOS and Linux)
            echo "# Ignored test baseline - $(date -u +%Y-%m-%dT%H:%M:%SZ)"
            echo "# Updated by: ignored-test-count.sh --update"
            for cat in brokenpipe feature infra protocol manual stress bug bare other; do
                echo "$cat=${counts[$cat]}"
            done
            echo "total=$total"
        } > "$BASELINE_FILE"
        echo -e "${GREEN}Baseline updated successfully.${NC}"
        ;;
    check)
        if [[ $total -gt $baseline_total ]]; then
            echo -e "${RED}ERROR: Ignored test count increased from $baseline_total to $total${NC}"
            echo ""
            echo "New ignores must be justified. If intentional, run:"
            echo "  ./scripts/ignored-test-count.sh --update"
            echo ""
            exit 1
        else
            echo -e "${GREEN}OK: Ignored test count ($total) is not higher than baseline ($baseline_total)${NC}"
            exit 0
        fi
        ;;
    show)
        # Just show the summary (already done above)
        if [[ $total -gt 0 ]]; then
            echo "Run with VERBOSE=1 for detailed breakdown:"
            echo "  VERBOSE=1 $0"
            echo ""
            echo "To update baseline:"
            echo "  $0 --update"
        fi
        ;;
esac
