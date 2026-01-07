#!/usr/bin/env bash
# Telemetry Runner - Compute hard probe deltas between base and head commits
# Part of the Analyzer Framework (see docs/ANALYZER_FRAMEWORK.md)
#
# Usage:
#   ./scripts/forensics/telemetry-runner.sh <base-sha> <head-sha>
#   ./scripts/forensics/telemetry-runner.sh --pr <pr-number>
#   ./scripts/forensics/telemetry-runner.sh --quick <base-sha> <head-sha>
#
# Options:
#   --quick       Skip expensive operations (cargo geiger, full test runs)
#   --pr <N>      Fetch base/head from GitHub PR (requires gh CLI)
#   --output-dir  Output directory for artifacts (default: ./artifacts/telemetry)
#
# Lane 1 (always available):
#   - tokei: LOC by module
#   - cargo clippy: warning count
#   - cargo test: test counts (passed/failed/ignored)
#   - Cargo.toml: dependency count
#
# Lane 2 (if available):
#   - cargo geiger: unsafe block count
#   - cargo tree -d: duplicate dependencies
#
# Output: YAML to stdout, artifacts to --output-dir

set -euo pipefail

# =============================================================================
# Configuration
# =============================================================================

PROJECT_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
WORKTREE_DIR="${PROJECT_ROOT}/.worktrees"
OUTPUT_DIR="${PROJECT_ROOT}/artifacts/telemetry"
QUICK_MODE=false
PR_MODE=false
PR_NUMBER=""

# Colors for stderr logging
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m'

# =============================================================================
# Logging (all to stderr to keep stdout clean for YAML)
# =============================================================================

log_info() {
    echo -e "${BLUE}[INFO]${NC} $*" >&2
}

log_warn() {
    echo -e "${YELLOW}[WARN]${NC} $*" >&2
}

log_error() {
    echo -e "${RED}[ERROR]${NC} $*" >&2
}

log_success() {
    echo -e "${GREEN}[OK]${NC} $*" >&2
}

log_progress() {
    echo -e "${BLUE}>>>${NC} $*" >&2
}

# =============================================================================
# Argument Parsing
# =============================================================================

usage() {
    cat >&2 <<EOF
Usage: $(basename "$0") [OPTIONS] <base-sha> <head-sha>
       $(basename "$0") --pr <pr-number>

Options:
  --quick         Skip expensive operations (geiger, full tests)
  --pr <N>        Fetch base/head from GitHub PR
  --output-dir    Output directory (default: ./artifacts/telemetry)
  -h, --help      Show this help message

Examples:
  $(basename "$0") abc1234 def5678
  $(basename "$0") --quick HEAD~5 HEAD
  $(basename "$0") --pr 123
EOF
    exit 1
}

BASE_SHA=""
HEAD_SHA=""

while [[ $# -gt 0 ]]; do
    case $1 in
        --quick)
            QUICK_MODE=true
            shift
            ;;
        --pr)
            PR_MODE=true
            PR_NUMBER="${2:-}"
            if [[ -z "$PR_NUMBER" ]]; then
                log_error "--pr requires a PR number"
                usage
            fi
            shift 2
            ;;
        --output-dir)
            OUTPUT_DIR="${2:-}"
            if [[ -z "$OUTPUT_DIR" ]]; then
                log_error "--output-dir requires a path"
                usage
            fi
            shift 2
            ;;
        -h|--help)
            usage
            ;;
        -*)
            log_error "Unknown option: $1"
            usage
            ;;
        *)
            if [[ -z "$BASE_SHA" ]]; then
                BASE_SHA="$1"
            elif [[ -z "$HEAD_SHA" ]]; then
                HEAD_SHA="$1"
            else
                log_error "Too many arguments"
                usage
            fi
            shift
            ;;
    esac
done

# =============================================================================
# PR Mode: Fetch SHAs from GitHub
# =============================================================================

if [[ "$PR_MODE" == "true" ]]; then
    log_progress "Fetching PR #${PR_NUMBER} info from GitHub..."

    if ! command -v gh &> /dev/null; then
        log_error "gh CLI not installed (required for --pr mode)"
        exit 1
    fi

    PR_JSON=$(gh pr view "$PR_NUMBER" --json baseRefName,headRefOid,baseRefOid 2>/dev/null || true)
    if [[ -z "$PR_JSON" ]]; then
        log_error "Failed to fetch PR #${PR_NUMBER}"
        exit 1
    fi

    BASE_SHA=$(echo "$PR_JSON" | jq -r '.baseRefOid')
    HEAD_SHA=$(echo "$PR_JSON" | jq -r '.headRefOid')

    log_info "PR #${PR_NUMBER}: base=${BASE_SHA:0:8} head=${HEAD_SHA:0:8}"
fi

# Validate we have both SHAs
if [[ -z "$BASE_SHA" ]] || [[ -z "$HEAD_SHA" ]]; then
    log_error "Both base and head SHAs are required"
    usage
fi

# Resolve short SHAs to full
cd "$PROJECT_ROOT"
BASE_SHA=$(git rev-parse "$BASE_SHA" 2>/dev/null || echo "$BASE_SHA")
HEAD_SHA=$(git rev-parse "$HEAD_SHA" 2>/dev/null || echo "$HEAD_SHA")

log_info "Base: ${BASE_SHA:0:12}"
log_info "Head: ${HEAD_SHA:0:12}"

# =============================================================================
# Tool Detection
# =============================================================================

detect_tools() {
    log_progress "Detecting available tools..."

    # Lane 1 tools (always expected)
    HAVE_TOKEI=false
    HAVE_CARGO=false

    # Lane 2 tools (optional)
    HAVE_GEIGER=false
    HAVE_TREE=true  # cargo tree is always available with cargo

    if command -v tokei &> /dev/null; then
        HAVE_TOKEI=true
        log_info "tokei: available"
    else
        log_warn "tokei: not installed (LOC metrics will be null)"
    fi

    if command -v cargo &> /dev/null; then
        HAVE_CARGO=true
        log_info "cargo: available"
    else
        log_error "cargo: not found (required)"
        exit 1
    fi

    if cargo geiger --version &> /dev/null 2>&1; then
        HAVE_GEIGER=true
        log_info "cargo-geiger: available"
    else
        log_warn "cargo-geiger: not installed (unsafe count will be null)"
    fi
}

# =============================================================================
# Worktree Management
# =============================================================================

setup_worktrees() {
    log_progress "Setting up git worktrees..."

    mkdir -p "$WORKTREE_DIR"

    # Clean up any existing worktrees from previous runs
    cleanup_worktrees 2>/dev/null || true

    # Create worktrees for base and head
    BASE_DIR="${WORKTREE_DIR}/base-${BASE_SHA:0:8}"
    HEAD_DIR="${WORKTREE_DIR}/head-${HEAD_SHA:0:8}"

    log_info "Creating base worktree at ${BASE_DIR}..."
    git worktree add --detach "$BASE_DIR" "$BASE_SHA" 2>/dev/null || {
        log_error "Failed to create base worktree for $BASE_SHA"
        exit 1
    }

    log_info "Creating head worktree at ${HEAD_DIR}..."
    git worktree add --detach "$HEAD_DIR" "$HEAD_SHA" 2>/dev/null || {
        log_error "Failed to create head worktree for $HEAD_SHA"
        cleanup_worktrees
        exit 1
    }

    log_success "Worktrees created"
}

cleanup_worktrees() {
    log_progress "Cleaning up worktrees..."

    # Must be in project root for git worktree commands
    cd "$PROJECT_ROOT"

    # Remove worktrees by directory (using variables if set, otherwise find any in worktree dir)
    local base_wt="${WORKTREE_DIR}/base-${BASE_SHA:0:8}"
    local head_wt="${WORKTREE_DIR}/head-${HEAD_SHA:0:8}"

    if [[ -n "${BASE_SHA:-}" ]] && [[ -d "$base_wt" ]]; then
        git worktree remove --force "$base_wt" 2>/dev/null || rm -rf "$base_wt" 2>/dev/null || true
    fi
    if [[ -n "${HEAD_SHA:-}" ]] && [[ -d "$head_wt" ]]; then
        git worktree remove --force "$head_wt" 2>/dev/null || rm -rf "$head_wt" 2>/dev/null || true
    fi

    # Clean up any orphaned worktrees in the directory
    if [[ -d "$WORKTREE_DIR" ]]; then
        for wt in "$WORKTREE_DIR"/*; do
            if [[ -d "$wt" ]]; then
                git worktree remove --force "$wt" 2>/dev/null || rm -rf "$wt" 2>/dev/null || true
            fi
        done
    fi

    # Prune any stale worktree entries
    git worktree prune 2>/dev/null || true

    # Remove worktree directory if empty
    rmdir "$WORKTREE_DIR" 2>/dev/null || true
}

# Trap to ensure cleanup on exit
trap cleanup_worktrees EXIT

# =============================================================================
# Metric Collection Functions
# =============================================================================

# Collect LOC using tokei
collect_loc() {
    local dir="$1"
    local prefix="$2"

    if [[ "$HAVE_TOKEI" != "true" ]]; then
        echo "null"
        return
    fi

    log_info "  Collecting LOC for $prefix..."

    local tokei_output
    tokei_output=$(tokei "$dir/crates" --output json 2>/dev/null || echo '{}')

    # Parse tokei JSON output
    local total_code
    total_code=$(echo "$tokei_output" | jq '.Total.code // 0' 2>/dev/null || echo "0")

    # Get per-crate breakdown
    local by_crate="{}"
    if [[ -d "$dir/crates" ]]; then
        by_crate="{"
        local first=true
        for crate_dir in "$dir/crates"/*/; do
            if [[ -d "$crate_dir" ]]; then
                local crate_name
                crate_name=$(basename "$crate_dir")
                local crate_loc
                crate_loc=$(tokei "$crate_dir" --output json 2>/dev/null | jq '.Total.code // 0' 2>/dev/null || echo "0")
                if [[ "$first" == "true" ]]; then
                    first=false
                else
                    by_crate+=", "
                fi
                by_crate+="\"$crate_name\": $crate_loc"
            fi
        done
        by_crate+="}"
    fi

    echo "{\"total\": $total_code, \"by_crate\": $by_crate}"
}

# Collect clippy warnings
collect_clippy() {
    local dir="$1"
    local prefix="$2"

    log_info "  Collecting clippy warnings for $prefix..."

    cd "$dir"
    local warning_count
    warning_count=$(cargo clippy --workspace --all-targets -- -W clippy::all 2>&1 | grep -c '^warning:' || echo "0")

    echo "$warning_count"
}

# Collect test counts
collect_tests() {
    local dir="$1"
    local prefix="$2"

    if [[ "$QUICK_MODE" == "true" ]]; then
        log_info "  Skipping tests for $prefix (--quick mode)"
        echo '{"passed": null, "failed": null, "ignored": null, "total": null}'
        return
    fi

    log_info "  Running tests for $prefix..."

    cd "$dir"
    local test_output
    test_output=$(RUST_TEST_THREADS=2 cargo test --workspace --lib -- --test-threads=2 2>&1 || true)

    # Parse test output (format: "test result: ok. X passed; Y failed; Z ignored")
    local results
    results=$(echo "$test_output" | grep -E '^[[:space:]]*test result:' || true)

    if [[ -z "$results" ]]; then
        echo '{"passed": 0, "failed": 0, "ignored": 0, "total": 0}'
        return
    fi

    local passed failed ignored
    passed=$(echo "$results" | awk '{sum += $4} END {print sum+0}')
    failed=$(echo "$results" | awk '{sum += $6} END {print sum+0}')
    ignored=$(echo "$results" | awk '{sum += $8} END {print sum+0}')
    local total=$((passed + failed + ignored))

    echo "{\"passed\": $passed, \"failed\": $failed, \"ignored\": $ignored, \"total\": $total}"
}

# Collect dependency count from Cargo.toml
collect_dependencies() {
    local dir="$1"
    local prefix="$2"

    log_info "  Counting dependencies for $prefix..."

    local dep_count=0

    # Count unique dependencies across all Cargo.toml files
    if [[ -f "$dir/Cargo.lock" ]]; then
        # Use Cargo.lock for accurate count (includes transitive)
        dep_count=$(grep -c '^\[\[package\]\]' "$dir/Cargo.lock" 2>/dev/null || echo "0")
    else
        # Fallback: count from Cargo.toml files
        for cargo_toml in "$dir"/crates/*/Cargo.toml "$dir/Cargo.toml"; do
            if [[ -f "$cargo_toml" ]]; then
                local crate_deps
                crate_deps=$(grep -cE '^\[dependencies|^\[dev-dependencies|^\[build-dependencies' "$cargo_toml" 2>/dev/null || echo "0")
                dep_count=$((dep_count + crate_deps))
            fi
        done
    fi

    echo "$dep_count"
}

# Collect unsafe block count using cargo-geiger
collect_unsafe() {
    local dir="$1"
    local prefix="$2"

    if [[ "$HAVE_GEIGER" != "true" ]] || [[ "$QUICK_MODE" == "true" ]]; then
        if [[ "$QUICK_MODE" == "true" ]]; then
            log_info "  Skipping geiger for $prefix (--quick mode)"
        fi
        echo "null"
        return
    fi

    log_info "  Running cargo-geiger for $prefix (this may take a while)..."

    cd "$dir"
    local geiger_output
    geiger_output=$(cargo geiger --all-features 2>&1 || true)

    # Parse geiger output for unsafe usage
    # Look for "used/unused unsafe in total" line
    local unsafe_count
    unsafe_count=$(echo "$geiger_output" | grep -oE '[0-9]+/[0-9]+ unsafe' | head -1 | cut -d'/' -f1 || echo "0")

    if [[ -z "$unsafe_count" ]]; then
        unsafe_count=0
    fi

    echo "$unsafe_count"
}

# Collect duplicate dependencies
collect_duplicates() {
    local dir="$1"
    local prefix="$2"

    if [[ "$QUICK_MODE" == "true" ]]; then
        log_info "  Skipping duplicate check for $prefix (--quick mode)"
        echo "null"
        return
    fi

    log_info "  Checking for duplicate dependencies for $prefix..."

    cd "$dir"
    local dup_count
    dup_count=$(cargo tree -d 2>/dev/null | grep -cE '^\w' || echo "0")

    echo "$dup_count"
}

# =============================================================================
# Collect All Metrics for a Commit
# =============================================================================

collect_metrics() {
    local dir="$1"
    local sha="$2"
    local prefix="$3"

    log_progress "Collecting metrics for $prefix (${sha:0:8})..."

    local loc clippy tests deps unsafe_count duplicates

    loc=$(collect_loc "$dir" "$prefix")
    clippy=$(collect_clippy "$dir" "$prefix")
    tests=$(collect_tests "$dir" "$prefix")
    deps=$(collect_dependencies "$dir" "$prefix")
    unsafe_count=$(collect_unsafe "$dir" "$prefix")
    duplicates=$(collect_duplicates "$dir" "$prefix")

    # Build JSON object
    cat <<EOF
{
  "sha": "$sha",
  "loc": $loc,
  "clippy_warnings": $clippy,
  "tests": $tests,
  "dependency_count": $deps,
  "unsafe_count": $unsafe_count,
  "duplicate_dependencies": $duplicates
}
EOF
}

# =============================================================================
# Delta Computation
# =============================================================================

compute_deltas() {
    local base_json="$1"
    local head_json="$2"

    log_progress "Computing deltas..."

    # Use jq to compute deltas
    jq -n \
        --argjson base "$base_json" \
        --argjson head "$head_json" \
        '{
            loc: (if $head.loc != null and $base.loc != null
                  then ($head.loc.total - $base.loc.total)
                  else null end),
            clippy_warnings: ($head.clippy_warnings - $base.clippy_warnings),
            test_count: (if $head.tests.total != null and $base.tests.total != null
                         then ($head.tests.total - $base.tests.total)
                         else null end),
            test_passed: (if $head.tests.passed != null and $base.tests.passed != null
                          then ($head.tests.passed - $base.tests.passed)
                          else null end),
            dependency_count: ($head.dependency_count - $base.dependency_count),
            unsafe_count: (if $head.unsafe_count != null and $base.unsafe_count != null
                           then ($head.unsafe_count - $base.unsafe_count)
                           else null end),
            duplicate_dependencies: (if $head.duplicate_dependencies != null and $base.duplicate_dependencies != null
                                     then ($head.duplicate_dependencies - $base.duplicate_dependencies)
                                     else null end)
        }'
}

# =============================================================================
# Output Generation
# =============================================================================

generate_yaml_output() {
    local base_json="$1"
    local head_json="$2"
    local deltas_json="$3"
    local timestamp
    timestamp=$(date -u +%Y-%m-%dT%H:%M:%SZ)

    # Convert JSON to YAML using jq and manual formatting
    # (yq is not always available, so we do it manually)

    cat <<EOF
# Telemetry Report
# Generated: $timestamp
# Base: ${BASE_SHA:0:12}
# Head: ${HEAD_SHA:0:12}
# Mode: $(if [[ "$QUICK_MODE" == "true" ]]; then echo "quick"; else echo "full"; fi)

base:
$(echo "$base_json" | jq -r '
  "  sha: \(.sha)",
  "  loc:",
  (if .loc == null then "    total: null\n    by_crate: null" else
    "    total: \(.loc.total)",
    "    by_crate:",
    (.loc.by_crate | to_entries[] | "      \(.key): \(.value)")
  end),
  "  clippy_warnings: \(.clippy_warnings)",
  "  tests:",
  "    passed: \(.tests.passed)",
  "    failed: \(.tests.failed)",
  "    ignored: \(.tests.ignored)",
  "    total: \(.tests.total)",
  "  dependency_count: \(.dependency_count)",
  "  unsafe_count: \(.unsafe_count)",
  "  duplicate_dependencies: \(.duplicate_dependencies)"
')

head:
$(echo "$head_json" | jq -r '
  "  sha: \(.sha)",
  "  loc:",
  (if .loc == null then "    total: null\n    by_crate: null" else
    "    total: \(.loc.total)",
    "    by_crate:",
    (.loc.by_crate | to_entries[] | "      \(.key): \(.value)")
  end),
  "  clippy_warnings: \(.clippy_warnings)",
  "  tests:",
  "    passed: \(.tests.passed)",
  "    failed: \(.tests.failed)",
  "    ignored: \(.tests.ignored)",
  "    total: \(.tests.total)",
  "  dependency_count: \(.dependency_count)",
  "  unsafe_count: \(.unsafe_count)",
  "  duplicate_dependencies: \(.duplicate_dependencies)"
')

deltas:
$(echo "$deltas_json" | jq -r '
  "  loc: \(.loc)",
  "  clippy_warnings: \(.clippy_warnings)",
  "  test_count: \(.test_count)",
  "  test_passed: \(.test_passed)",
  "  dependency_count: \(.dependency_count)",
  "  unsafe_count: \(.unsafe_count)",
  "  duplicate_dependencies: \(.duplicate_dependencies)"
')

metadata:
  generated_at: $timestamp
  quick_mode: $QUICK_MODE
  tools:
    tokei: $HAVE_TOKEI
    cargo_geiger: $HAVE_GEIGER
EOF
}

# =============================================================================
# Main Execution
# =============================================================================

main() {
    log_progress "Starting telemetry collection..."
    log_info "Quick mode: $QUICK_MODE"

    # Detect tools
    detect_tools

    # Setup worktrees
    setup_worktrees

    # Create output directory
    mkdir -p "$OUTPUT_DIR"

    # Collect metrics for base
    BASE_JSON=$(collect_metrics "$BASE_DIR" "$BASE_SHA" "base")
    echo "$BASE_JSON" > "${OUTPUT_DIR}/base-metrics.json"
    log_success "Base metrics collected"

    # Collect metrics for head
    HEAD_JSON=$(collect_metrics "$HEAD_DIR" "$HEAD_SHA" "head")
    echo "$HEAD_JSON" > "${OUTPUT_DIR}/head-metrics.json"
    log_success "Head metrics collected"

    # Compute deltas
    DELTAS_JSON=$(compute_deltas "$BASE_JSON" "$HEAD_JSON")
    echo "$DELTAS_JSON" > "${OUTPUT_DIR}/deltas.json"
    log_success "Deltas computed"

    # Generate YAML output to stdout
    log_progress "Generating YAML output..."
    generate_yaml_output "$BASE_JSON" "$HEAD_JSON" "$DELTAS_JSON"

    # Also save YAML to file
    generate_yaml_output "$BASE_JSON" "$HEAD_JSON" "$DELTAS_JSON" > "${OUTPUT_DIR}/telemetry-report.yaml"

    log_success "Telemetry collection complete"
    log_info "Artifacts saved to: ${OUTPUT_DIR}/"
    log_info "  - base-metrics.json"
    log_info "  - head-metrics.json"
    log_info "  - deltas.json"
    log_info "  - telemetry-report.yaml"
}

main
