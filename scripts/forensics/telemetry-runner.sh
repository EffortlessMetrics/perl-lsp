#!/usr/bin/env bash
# Telemetry Runner - Compute hard probe deltas between base and head commits
# Part of the Forensics Framework
#
# Usage:
#   ./scripts/forensics/telemetry-runner.sh <base-sha> [head-sha]
#   ./scripts/forensics/telemetry-runner.sh --pr <pr-number>
#   ./scripts/forensics/telemetry-runner.sh --quick <base-sha> <head-sha>
#   ./scripts/forensics/telemetry-runner.sh --full <base-sha> <head-sha>
#   ./scripts/forensics/telemetry-runner.sh --research <base-sha> <head-sha>
#
# Modes:
#   --quick         Always-on tools only (fmt, clippy, tests, audit, shellcheck, actionlint)
#   --full          Exhibit-grade analysis (adds semver-checks, geiger, typos, etc.)
#   --research      Research mode (full + rust-code-analysis, cargo-modules, cargo-udeps)
#
# Options:
#   --pr <N>        Fetch base/head from GitHub PR (requires gh CLI)
#   --output-dir    Output directory for artifacts (default: ./artifacts/telemetry)
#   -h, --help      Show this help message
#
# Always-on tools (quick mode):
#   - cargo fmt --check: pass/fail
#   - cargo clippy: warning count
#   - cargo test: pass/fail/count
#   - cargo audit: advisory count (if available)
#   - shellcheck: issue count for changed scripts (if available)
#   - actionlint: issue count for changed workflows (if available)
#
# Exhibit-grade tools (full mode adds):
#   - cargo semver-checks: breaking changes (if available)
#   - cargo geiger: unsafe block count (if available)
#   - typos: typo count in docs and identifiers (if available)
#   - Test count delta: Count test functions in diff
#   - Dependency delta: Parse Cargo.lock changes
#
# Research tools (research mode adds):
#   - rust-code-analysis: complexity metrics (if available)
#   - cargo-modules: module graph analysis (if available)
#   - cargo-udeps: unused dependency detection (if available)
#
# Output: YAML to stdout, artifacts to --output-dir

set -euo pipefail

# =============================================================================
# Configuration
# =============================================================================

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"
WORKTREE_DIR="${PROJECT_ROOT}/.worktrees"
OUTPUT_DIR="${PROJECT_ROOT}/artifacts/telemetry"
MODE="quick"  # Default to quick mode
PR_MODE=false
PR_NUMBER=""

# Timeouts (in seconds)
TIMEOUT_FMT=60
TIMEOUT_CLIPPY=300
TIMEOUT_TEST=600
TIMEOUT_AUDIT=120
TIMEOUT_SEMVER=300
TIMEOUT_GEIGER=600
TIMEOUT_SHELLCHECK=60
TIMEOUT_ACTIONLINT=60
TIMEOUT_TYPOS=120
TIMEOUT_RCA=300
TIMEOUT_MODULES=120
TIMEOUT_UDEPS=300

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
Usage: $(basename "$0") [OPTIONS] <base-sha> [head-sha]
       $(basename "$0") --pr <pr-number>

Run static analysis tools and compute deltas between base and head commits.

Options:
  --quick           Quick mode: always-on tools only (default)
  --full            Full mode: exhibit-grade analysis (slower)
  --research        Research mode: full + complexity/dependency tools (slowest)
  --pr <N>          Fetch base/head from GitHub PR
  --output-dir DIR  Output directory (default: ./artifacts/telemetry)
  -h, --help        Show this help message

Quick mode tools:
  - cargo fmt --check
  - cargo clippy
  - cargo test
  - cargo audit (if available)
  - shellcheck (if scripts/ changed, if available)
  - actionlint (if workflows/ changed, if available)

Full mode adds:
  - cargo semver-checks (if available)
  - cargo geiger (if available)
  - typos (if available)
  - test function count delta
  - dependency delta analysis

Research mode adds:
  - rust-code-analysis (if available)
  - cargo-modules (if available)
  - cargo-udeps (if available)

Examples:
  $(basename "$0") abc1234 def5678
  $(basename "$0") --quick HEAD~5 HEAD
  $(basename "$0") --full origin/master HEAD
  $(basename "$0") --research origin/master HEAD
  $(basename "$0") --pr 123
EOF
    exit 1
}

BASE_SHA=""
HEAD_SHA=""

while [[ $# -gt 0 ]]; do
    case $1 in
        --quick)
            MODE="quick"
            shift
            ;;
        --full)
            MODE="full"
            shift
            ;;
        --research)
            MODE="research"
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

    PR_JSON=$(gh pr view "$PR_NUMBER" --json baseRefOid,headRefOid 2>/dev/null || true)
    if [[ -z "$PR_JSON" ]]; then
        log_error "Failed to fetch PR #${PR_NUMBER}"
        exit 1
    fi

    BASE_SHA=$(echo "$PR_JSON" | jq -r '.baseRefOid')
    HEAD_SHA=$(echo "$PR_JSON" | jq -r '.headRefOid')

    log_info "PR #${PR_NUMBER}: base=${BASE_SHA:0:8} head=${HEAD_SHA:0:8}"
fi

# Default head to HEAD if not specified
if [[ -z "$HEAD_SHA" ]]; then
    HEAD_SHA="HEAD"
fi

# Validate we have base SHA
if [[ -z "$BASE_SHA" ]]; then
    log_error "Base SHA is required"
    usage
fi

# Resolve SHAs to full form
cd "$PROJECT_ROOT"
BASE_SHA=$(git rev-parse "$BASE_SHA" 2>/dev/null || echo "$BASE_SHA")
HEAD_SHA=$(git rev-parse "$HEAD_SHA" 2>/dev/null || echo "$HEAD_SHA")

log_info "Mode: $MODE"
log_info "Base: ${BASE_SHA:0:12}"
log_info "Head: ${HEAD_SHA:0:12}"

# =============================================================================
# Tool Detection
# =============================================================================

# Tool availability flags
HAVE_CARGO=false
HAVE_AUDIT=false
HAVE_SEMVER=false
HAVE_GEIGER=false
HAVE_SHELLCHECK=false
HAVE_ACTIONLINT=false
HAVE_TYPOS=false
HAVE_RCA=false
HAVE_MODULES=false
HAVE_UDEPS=false

detect_tools() {
    log_progress "Detecting available tools..."

    if command -v cargo &> /dev/null; then
        HAVE_CARGO=true
        log_info "cargo: available"
    else
        log_error "cargo: not found (required)"
        exit 1
    fi

    if cargo audit --version &> /dev/null 2>&1; then
        HAVE_AUDIT=true
        log_info "cargo-audit: available"
    else
        log_warn "cargo-audit: not installed"
    fi

    if cargo semver-checks --version &> /dev/null 2>&1; then
        HAVE_SEMVER=true
        log_info "cargo-semver-checks: available"
    else
        log_warn "cargo-semver-checks: not installed"
    fi

    if cargo geiger --version &> /dev/null 2>&1; then
        HAVE_GEIGER=true
        log_info "cargo-geiger: available"
    else
        log_warn "cargo-geiger: not installed"
    fi

    if command -v shellcheck &> /dev/null; then
        HAVE_SHELLCHECK=true
        log_info "shellcheck: available"
    else
        log_warn "shellcheck: not installed"
    fi

    if command -v actionlint &> /dev/null; then
        HAVE_ACTIONLINT=true
        log_info "actionlint: available"
    else
        log_warn "actionlint: not installed"
    fi

    if command -v typos &> /dev/null; then
        HAVE_TYPOS=true
        log_info "typos: available"
    else
        log_warn "typos: not installed"
    fi

    if command -v rust-code-analysis &> /dev/null; then
        HAVE_RCA=true
        log_info "rust-code-analysis: available"
    else
        log_warn "rust-code-analysis: not installed"
    fi

    if cargo modules --version &> /dev/null 2>&1; then
        HAVE_MODULES=true
        log_info "cargo-modules: available"
    else
        log_warn "cargo-modules: not installed"
    fi

    if cargo udeps --version &> /dev/null 2>&1; then
        HAVE_UDEPS=true
        log_info "cargo-udeps: available"
    else
        log_warn "cargo-udeps: not installed"
    fi
}

# =============================================================================
# Worktree Management
# =============================================================================

BASE_DIR=""
HEAD_DIR=""

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

    # Remove worktrees by directory
    if [[ -n "${BASE_DIR:-}" ]] && [[ -d "$BASE_DIR" ]]; then
        git worktree remove --force "$BASE_DIR" 2>/dev/null || rm -rf "$BASE_DIR" 2>/dev/null || true
    fi
    if [[ -n "${HEAD_DIR:-}" ]] && [[ -d "$HEAD_DIR" ]]; then
        git worktree remove --force "$HEAD_DIR" 2>/dev/null || rm -rf "$HEAD_DIR" 2>/dev/null || true
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
# Timeout Wrapper
# =============================================================================

# Run command with timeout protection
run_with_timeout() {
    local timeout_secs="$1"
    shift
    local cmd="$*"

    if command -v timeout &> /dev/null; then
        timeout "$timeout_secs" bash -c "$cmd" 2>&1 || true
    else
        # macOS fallback - use perl
        perl -e "alarm $timeout_secs; exec @ARGV" -- bash -c "$cmd" 2>&1 || true
    fi
}

# =============================================================================
# Tool Runners - Always-on (Quick Mode)
# =============================================================================

# cargo fmt --check
run_fmt_check() {
    local dir="$1"
    log_info "  Running cargo fmt --check..."

    cd "$dir"
    local output
    output=$(run_with_timeout "$TIMEOUT_FMT" "cargo fmt --check --all 2>&1")
    local status="pass"
    if echo "$output" | grep -q "Diff in"; then
        status="fail"
    fi
    echo "$status"
}

# cargo clippy - count warnings
run_clippy() {
    local dir="$1"
    log_info "  Running cargo clippy..."

    cd "$dir"
    local output
    output=$(run_with_timeout "$TIMEOUT_CLIPPY" "cargo clippy --workspace --lib -- -D warnings 2>&1" || true)

    # Count warning lines
    local warning_count
    warning_count=$(echo "$output" | grep -c '^warning:' || echo "0")

    # Determine status
    local status="pass"
    if echo "$output" | grep -q "error\["; then
        status="fail"
    elif [[ $warning_count -gt 0 ]]; then
        status="warn"
    fi

    echo "${status}|${warning_count}"
}

# cargo test - collect pass/fail counts
run_tests() {
    local dir="$1"
    log_info "  Running cargo test..."

    cd "$dir"
    local output
    output=$(run_with_timeout "$TIMEOUT_TEST" "RUST_TEST_THREADS=2 cargo test --workspace --lib --no-fail-fast -- --test-threads=2 2>&1" || true)

    # Parse test results
    # Look for lines like: "test result: ok. 123 passed; 0 failed; 5 ignored"
    local passed=0 failed=0 ignored=0
    local status="pass"

    while IFS= read -r line; do
        if [[ "$line" =~ test\ result:.*([0-9]+)\ passed.*([0-9]+)\ failed.*([0-9]+)\ ignored ]]; then
            passed=$((passed + ${BASH_REMATCH[1]}))
            failed=$((failed + ${BASH_REMATCH[2]}))
            ignored=$((ignored + ${BASH_REMATCH[3]}))
        fi
    done <<< "$output"

    if [[ $failed -gt 0 ]]; then
        status="fail"
    fi

    echo "${status}|${passed}|${failed}|${ignored}"
}

# cargo audit - count advisories
run_audit() {
    local dir="$1"

    if [[ "$HAVE_AUDIT" != "true" ]]; then
        echo "unavailable|0"
        return
    fi

    log_info "  Running cargo audit..."

    cd "$dir"
    local output
    output=$(run_with_timeout "$TIMEOUT_AUDIT" "cargo audit 2>&1" || true)

    # Count advisories
    local advisory_count
    advisory_count=$(echo "$output" | grep -c "^Crate:" || echo "0")

    echo "available|${advisory_count}"
}

# shellcheck - check shell scripts (conditional on scripts/ changes)
run_shellcheck() {
    local base_dir="$1"
    local head_dir="$2"

    if [[ "$HAVE_SHELLCHECK" != "true" ]]; then
        echo "unavailable|0|0"
        return
    fi

    # Check if scripts/ directory changed between base and head
    cd "$PROJECT_ROOT"
    local changed_scripts
    changed_scripts=$(git diff --name-only "$BASE_SHA" "$HEAD_SHA" | grep '^scripts/.*\.sh$' || true)

    if [[ -z "$changed_scripts" ]]; then
        echo "skipped|0|0"
        return
    fi

    log_info "  Running shellcheck on changed scripts..."

    # Run shellcheck on base
    local base_count=0
    cd "$base_dir"
    if [[ -d scripts ]]; then
        local base_output
        base_output=$(run_with_timeout "$TIMEOUT_SHELLCHECK" "find scripts -type f -name '*.sh' -exec shellcheck -f gcc {} + 2>&1" || true)
        base_count=$(echo "$base_output" | grep -cE '^[^:]+:[0-9]+:[0-9]+: (error|warning):' || echo "0")
    fi

    # Run shellcheck on head
    local head_count=0
    cd "$head_dir"
    if [[ -d scripts ]]; then
        local head_output
        head_output=$(run_with_timeout "$TIMEOUT_SHELLCHECK" "find scripts -type f -name '*.sh' -exec shellcheck -f gcc {} + 2>&1" || true)
        head_count=$(echo "$head_output" | grep -cE '^[^:]+:[0-9]+:[0-9]+: (error|warning):' || echo "0")
    fi

    echo "available|${base_count}|${head_count}"
}

# actionlint - check GitHub workflow files (conditional on .github/workflows/ changes)
run_actionlint() {
    local base_dir="$1"
    local head_dir="$2"

    if [[ "$HAVE_ACTIONLINT" != "true" ]]; then
        echo "unavailable|0|0"
        return
    fi

    # Check if .github/workflows/ changed between base and head
    cd "$PROJECT_ROOT"
    local changed_workflows
    changed_workflows=$(git diff --name-only "$BASE_SHA" "$HEAD_SHA" | grep '^\.github/workflows/.*\.ya\?ml$' || true)

    if [[ -z "$changed_workflows" ]]; then
        echo "skipped|0|0"
        return
    fi

    log_info "  Running actionlint on changed workflows..."

    # Run actionlint on base
    local base_count=0
    cd "$base_dir"
    if [[ -d .github/workflows ]]; then
        local base_output
        base_output=$(run_with_timeout "$TIMEOUT_ACTIONLINT" "actionlint .github/workflows/*.{yml,yaml} 2>&1" || true)
        base_count=$(echo "$base_output" | grep -cE '^\.github/workflows/.*:' || echo "0")
    fi

    # Run actionlint on head
    local head_count=0
    cd "$head_dir"
    if [[ -d .github/workflows ]]; then
        local head_output
        head_output=$(run_with_timeout "$TIMEOUT_ACTIONLINT" "actionlint .github/workflows/*.{yml,yaml} 2>&1" || true)
        head_count=$(echo "$head_output" | grep -cE '^\.github/workflows/.*:' || echo "0")
    fi

    echo "available|${base_count}|${head_count}"
}

# =============================================================================
# Tool Runners - Exhibit-Grade (Full Mode)
# =============================================================================

# cargo semver-checks - detect breaking changes
run_semver_checks() {
    local dir="$1"

    if [[ "$HAVE_SEMVER" != "true" ]]; then
        echo "unavailable|[]"
        return
    fi

    log_info "  Running cargo semver-checks..."

    cd "$dir"
    local output
    output=$(run_with_timeout "$TIMEOUT_SEMVER" "cargo semver-checks check-release 2>&1" || true)

    # Extract breaking changes
    local breaking_changes="[]"
    local changes_list=""

    while IFS= read -r line; do
        if [[ "$line" =~ ^---\ (.*) ]]; then
            local change="${BASH_REMATCH[1]}"
            if [[ -n "$changes_list" ]]; then
                changes_list+=", "
            fi
            changes_list+="\"$change\""
        fi
    done <<< "$output"

    if [[ -n "$changes_list" ]]; then
        breaking_changes="[$changes_list]"
    fi

    echo "available|${breaking_changes}"
}

# cargo geiger - count unsafe blocks
run_geiger() {
    local dir="$1"

    if [[ "$HAVE_GEIGER" != "true" ]]; then
        echo "unavailable|0"
        return
    fi

    log_info "  Running cargo geiger (this may take a while)..."

    cd "$dir"
    local output
    output=$(run_with_timeout "$TIMEOUT_GEIGER" "cargo geiger --output-format Json 2>&1" || true)

    # Parse JSON output for unsafe count
    local unsafe_count=0
    if echo "$output" | jq -e '.packages' &>/dev/null; then
        unsafe_count=$(echo "$output" | jq '[.packages[].unsafety.used.functions.unsafe] | add // 0' 2>/dev/null || echo "0")
    else
        # Fallback: parse text output
        unsafe_count=$(echo "$output" | grep -oE '[0-9]+/[0-9]+ unsafe' | head -1 | cut -d'/' -f1 || echo "0")
    fi

    echo "available|${unsafe_count}"
}

# Count test functions in source
count_test_functions() {
    local dir="$1"
    log_info "  Counting test functions..."

    # Count #[test] attributes
    local count
    count=$(grep -r '#\[test\]' "$dir/crates" 2>/dev/null | wc -l | tr -d ' ' || echo "0")
    echo "$count"
}

# Parse Cargo.lock for dependency analysis
analyze_dependencies() {
    local base_dir="$1"
    local head_dir="$2"
    log_info "  Analyzing dependency changes..."

    local base_lock="$base_dir/Cargo.lock"
    local head_lock="$head_dir/Cargo.lock"

    # Extract package names from Cargo.lock
    extract_packages() {
        local lockfile="$1"
        if [[ -f "$lockfile" ]]; then
            grep '^name = "' "$lockfile" | sed 's/name = "\([^"]*\)"/\1/' | sort -u
        fi
    }

    local base_pkgs head_pkgs
    base_pkgs=$(extract_packages "$base_lock")
    head_pkgs=$(extract_packages "$head_lock")

    # Compute differences
    local added removed
    added=$(comm -13 <(echo "$base_pkgs") <(echo "$head_pkgs") | tr '\n' '|')
    removed=$(comm -23 <(echo "$base_pkgs") <(echo "$head_pkgs") | tr '\n' '|')

    # Find updated packages (same name, different version)
    # This requires more sophisticated parsing
    local updated=""

    # For now, identify packages that changed version
    if [[ -f "$base_lock" ]] && [[ -f "$head_lock" ]]; then
        # Get name+version pairs
        extract_versions() {
            local lockfile="$1"
            awk '/^\[\[package\]\]/{name="";version=""} /^name = /{gsub(/"/, "", $3); name=$3} /^version = /{gsub(/"/, "", $3); version=$3; if(name!="") print name ":" version}' "$lockfile" | sort
        }

        local base_versions head_versions
        base_versions=$(extract_versions "$base_lock")
        head_versions=$(extract_versions "$head_lock")

        # Find packages with version changes
        while IFS= read -r pkg; do
            local name="${pkg%%:*}"
            local base_ver head_ver
            base_ver=$(echo "$base_versions" | grep "^${name}:" | head -1 | cut -d: -f2 || true)
            head_ver=$(echo "$head_versions" | grep "^${name}:" | head -1 | cut -d: -f2 || true)

            if [[ -n "$base_ver" ]] && [[ -n "$head_ver" ]] && [[ "$base_ver" != "$head_ver" ]]; then
                if [[ -n "$updated" ]]; then
                    updated+=" "
                fi
                updated+="${name}:${base_ver}->${head_ver}"
            fi
        done < <(echo "$base_pkgs" | grep -Fx -f <(echo "$head_pkgs"))
    fi

    echo "${added}|${removed}|${updated}"
}

# typos - check for typos in docs and identifiers (full mode only)
run_typos() {
    local dir="$1"

    if [[ "$HAVE_TYPOS" != "true" ]]; then
        echo "unavailable|0"
        return
    fi

    log_info "  Running typos..."

    cd "$dir"
    local output
    output=$(run_with_timeout "$TIMEOUT_TYPOS" "typos --format brief 2>&1" || true)

    # Count typo lines
    local typo_count
    typo_count=$(echo "$output" | grep -c '^\S\+:' || echo "0")

    echo "available|${typo_count}"
}

# =============================================================================
# Tool Runners - Research Mode
# =============================================================================

# rust-code-analysis - complexity metrics
run_rust_code_analysis() {
    local dir="$1"

    if [[ "$HAVE_RCA" != "true" ]]; then
        echo "unavailable|0|0|0"
        return
    fi

    log_info "  Running rust-code-analysis..."

    cd "$dir"
    local output
    output=$(run_with_timeout "$TIMEOUT_RCA" "rust-code-analysis --metrics -O json crates/ 2>&1" || true)

    # Parse JSON for complexity metrics
    local cyclomatic=0 cognitive=0 sloc=0
    if echo "$output" | jq -e '.metrics' &>/dev/null; then
        cyclomatic=$(echo "$output" | jq '[.metrics[] | .spaces[] | .metrics.cyclomatic.sum] | add // 0' 2>/dev/null || echo "0")
        cognitive=$(echo "$output" | jq '[.metrics[] | .spaces[] | .metrics.cognitive.sum] | add // 0' 2>/dev/null || echo "0")
        sloc=$(echo "$output" | jq '[.metrics[] | .spaces[] | .metrics.loc.sloc] | add // 0' 2>/dev/null || echo "0")
    fi

    echo "available|${cyclomatic}|${cognitive}|${sloc}"
}

# cargo-modules - module graph analysis
run_cargo_modules() {
    local dir="$1"

    if [[ "$HAVE_MODULES" != "true" ]]; then
        echo "unavailable|0"
        return
    fi

    log_info "  Running cargo-modules..."

    cd "$dir"
    local output
    output=$(run_with_timeout "$TIMEOUT_MODULES" "cargo modules graph --lib 2>&1" || true)

    # Count module declarations
    local module_count
    module_count=$(echo "$output" | grep -c '^\s*mod ' || echo "0")

    echo "available|${module_count}"
}

# cargo-udeps - unused dependency detection
run_cargo_udeps() {
    local dir="$1"

    if [[ "$HAVE_UDEPS" != "true" ]]; then
        echo "unavailable|0"
        return
    fi

    log_info "  Running cargo-udeps (this may take a while)..."

    cd "$dir"
    local output
    output=$(run_with_timeout "$TIMEOUT_UDEPS" "cargo +nightly udeps --all-targets 2>&1" || true)

    # Count unused dependencies
    local unused_count
    unused_count=$(echo "$output" | grep -c '^unused ' || echo "0")

    echo "available|${unused_count}"
}

# =============================================================================
# Collect All Metrics for a Commit
# =============================================================================

collect_metrics() {
    local dir="$1"
    local prefix="$2"

    log_progress "Collecting metrics for $prefix..."

    # Always-on tools
    local fmt_result clippy_result test_result audit_result
    fmt_result=$(run_fmt_check "$dir")
    clippy_result=$(run_clippy "$dir")
    test_result=$(run_tests "$dir")
    audit_result=$(run_audit "$dir")

    # Parse results
    local clippy_status clippy_warnings
    IFS='|' read -r clippy_status clippy_warnings <<< "$clippy_result"

    local test_status test_passed test_failed test_ignored
    IFS='|' read -r test_status test_passed test_failed test_ignored <<< "$test_result"

    local audit_available audit_advisories
    IFS='|' read -r audit_available audit_advisories <<< "$audit_result"

    # Full mode tools
    local semver_available="false" semver_breaking="[]"
    local geiger_available="false" geiger_unsafe="0"
    local typos_available="false" typos_count="0"
    local test_count="0"

    if [[ "$MODE" == "full" ]] || [[ "$MODE" == "research" ]]; then
        local semver_result geiger_result typos_result
        semver_result=$(run_semver_checks "$dir")
        geiger_result=$(run_geiger "$dir")
        typos_result=$(run_typos "$dir")
        test_count=$(count_test_functions "$dir")

        IFS='|' read -r semver_available semver_breaking <<< "$semver_result"
        IFS='|' read -r geiger_available geiger_unsafe <<< "$geiger_result"
        IFS='|' read -r typos_available typos_count <<< "$typos_result"
    fi

    # Research mode tools
    local rca_available="false" rca_cyclomatic="0" rca_cognitive="0" rca_sloc="0"
    local modules_available="false" modules_count="0"
    local udeps_available="false" udeps_count="0"

    if [[ "$MODE" == "research" ]]; then
        local rca_result modules_result udeps_result
        rca_result=$(run_rust_code_analysis "$dir")
        modules_result=$(run_cargo_modules "$dir")
        udeps_result=$(run_cargo_udeps "$dir")

        IFS='|' read -r rca_available rca_cyclomatic rca_cognitive rca_sloc <<< "$rca_result"
        IFS='|' read -r modules_available modules_count <<< "$modules_result"
        IFS='|' read -r udeps_available udeps_count <<< "$udeps_result"
    fi

    # Output JSON
    cat <<EOF
{
  "fmt": "$fmt_result",
  "clippy": {
    "status": "$clippy_status",
    "warnings": $clippy_warnings
  },
  "test": {
    "status": "$test_status",
    "passed": $test_passed,
    "failed": $test_failed,
    "ignored": $test_ignored
  },
  "audit": {
    "available": $(if [[ "$audit_available" == "available" ]]; then echo "true"; else echo "false"; fi),
    "advisories": $audit_advisories
  },
  "semver": {
    "available": $(if [[ "$semver_available" == "available" ]]; then echo "true"; else echo "false"; fi),
    "breaking_changes": $semver_breaking
  },
  "geiger": {
    "available": $(if [[ "$geiger_available" == "available" ]]; then echo "true"; else echo "false"; fi),
    "unsafe_count": $geiger_unsafe
  },
  "typos": {
    "available": $(if [[ "$typos_available" == "available" ]]; then echo "true"; else echo "false"; fi),
    "count": $typos_count
  },
  "rust_code_analysis": {
    "available": $(if [[ "$rca_available" == "available" ]]; then echo "true"; else echo "false"; fi),
    "cyclomatic": $rca_cyclomatic,
    "cognitive": $rca_cognitive,
    "sloc": $rca_sloc
  },
  "cargo_modules": {
    "available": $(if [[ "$modules_available" == "available" ]]; then echo "true"; else echo "false"; fi),
    "module_count": $modules_count
  },
  "cargo_udeps": {
    "available": $(if [[ "$udeps_available" == "available" ]]; then echo "true"; else echo "false"; fi),
    "unused_count": $udeps_count
  },
  "test_count": $test_count
}
EOF
}

# =============================================================================
# Delta Computation
# =============================================================================

compute_verdict() {
    local base_json="$1"
    local head_json="$2"

    local regressions="" improvements=""

    # Check clippy
    local base_warnings head_warnings
    base_warnings=$(echo "$base_json" | jq '.clippy.warnings')
    head_warnings=$(echo "$head_json" | jq '.clippy.warnings')
    if [[ $head_warnings -gt $base_warnings ]]; then
        regressions+="clippy "
    elif [[ $head_warnings -lt $base_warnings ]]; then
        improvements+="clippy "
    fi

    # Check tests
    local base_failed head_failed
    base_failed=$(echo "$base_json" | jq '.test.failed')
    head_failed=$(echo "$head_json" | jq '.test.failed')
    if [[ $head_failed -gt $base_failed ]]; then
        regressions+="tests "
    elif [[ $head_failed -lt $base_failed ]]; then
        improvements+="tests "
    fi

    # Check audit
    local base_audit head_audit
    base_audit=$(echo "$base_json" | jq '.audit.advisories')
    head_audit=$(echo "$head_json" | jq '.audit.advisories')
    if [[ $head_audit -gt $base_audit ]]; then
        regressions+="audit "
    elif [[ $head_audit -lt $base_audit ]]; then
        improvements+="audit "
    fi

    # Check geiger (full mode only)
    if [[ "$MODE" == "full" ]]; then
        local base_unsafe head_unsafe
        base_unsafe=$(echo "$base_json" | jq '.geiger.unsafe_count')
        head_unsafe=$(echo "$head_json" | jq '.geiger.unsafe_count')
        if [[ $head_unsafe -gt $base_unsafe ]]; then
            regressions+="geiger "
        elif [[ $head_unsafe -lt $base_unsafe ]]; then
            improvements+="geiger "
        fi
    fi

    # Determine verdict
    local verdict="pass"
    local head_fmt head_test_status
    head_fmt=$(echo "$head_json" | jq -r '.fmt')
    head_test_status=$(echo "$head_json" | jq -r '.test.status')

    if [[ "$head_fmt" == "fail" ]] || [[ "$head_test_status" == "fail" ]]; then
        verdict="fail"
    elif [[ -n "$regressions" ]]; then
        verdict="warn"
    fi

    echo "${verdict}|${regressions}|${improvements}"
}

# =============================================================================
# YAML Output Generation
# =============================================================================

generate_yaml_output() {
    local base_json="$1"
    local head_json="$2"
    local dep_delta="$3"
    local shellcheck_result="${4:-unavailable|0|0}"
    local actionlint_result="${5:-unavailable|0|0}"

    local timestamp
    timestamp=$(date -u +%Y-%m-%dT%H:%M:%SZ)

    # Compute verdict
    local verdict_result regressions improvements
    IFS='|' read -r verdict_result regressions improvements <<< "$(compute_verdict "$base_json" "$head_json")"

    # Parse dependency delta
    local deps_added deps_removed deps_updated
    IFS='|' read -r deps_added deps_removed deps_updated <<< "$dep_delta"

    # Parse shellcheck/actionlint results
    local shellcheck_status shellcheck_base shellcheck_head
    IFS='|' read -r shellcheck_status shellcheck_base shellcheck_head <<< "$shellcheck_result"
    local shellcheck_delta=$((shellcheck_head - shellcheck_base))

    local actionlint_status actionlint_base actionlint_head
    IFS='|' read -r actionlint_status actionlint_base actionlint_head <<< "$actionlint_result"
    local actionlint_delta=$((actionlint_head - actionlint_base))

    # Format arrays
    format_array() {
        local items="$1"
        if [[ -z "$items" ]]; then
            echo "[]"
        else
            echo "$items" | tr '|' '\n' | grep -v '^$' | sed 's/^/    - /' | head -c -1
        fi
    }

    # Extract values from JSON
    local base_fmt head_fmt
    base_fmt=$(echo "$base_json" | jq -r '.fmt')
    head_fmt=$(echo "$head_json" | jq -r '.fmt')

    local base_clippy_warnings head_clippy_warnings
    base_clippy_warnings=$(echo "$base_json" | jq '.clippy.warnings')
    head_clippy_warnings=$(echo "$head_json" | jq '.clippy.warnings')
    local clippy_delta=$((head_clippy_warnings - base_clippy_warnings))

    local base_test_passed head_test_passed base_test_failed head_test_failed
    base_test_passed=$(echo "$base_json" | jq '.test.passed')
    head_test_passed=$(echo "$head_json" | jq '.test.passed')
    base_test_failed=$(echo "$base_json" | jq '.test.failed')
    head_test_failed=$(echo "$head_json" | jq '.test.failed')

    local audit_available base_audit head_audit audit_delta
    audit_available=$(echo "$head_json" | jq '.audit.available')
    base_audit=$(echo "$base_json" | jq '.audit.advisories')
    head_audit=$(echo "$head_json" | jq '.audit.advisories')
    audit_delta=$((head_audit - base_audit))

    # Full mode metrics
    local semver_available="false" semver_breaking="[]"
    local geiger_available="false" base_unsafe="0" head_unsafe="0" geiger_delta="0"
    local typos_available="false" base_typos="0" head_typos="0" typos_delta="0"
    local base_test_count="0" head_test_count="0" test_count_delta="0"

    if [[ "$MODE" == "full" ]] || [[ "$MODE" == "research" ]]; then
        semver_available=$(echo "$head_json" | jq '.semver.available')
        semver_breaking=$(echo "$head_json" | jq -c '.semver.breaking_changes')

        geiger_available=$(echo "$head_json" | jq '.geiger.available')
        base_unsafe=$(echo "$base_json" | jq '.geiger.unsafe_count')
        head_unsafe=$(echo "$head_json" | jq '.geiger.unsafe_count')
        geiger_delta=$((head_unsafe - base_unsafe))

        typos_available=$(echo "$head_json" | jq '.typos.available')
        base_typos=$(echo "$base_json" | jq '.typos.count')
        head_typos=$(echo "$head_json" | jq '.typos.count')
        typos_delta=$((head_typos - base_typos))

        base_test_count=$(echo "$base_json" | jq '.test_count')
        head_test_count=$(echo "$head_json" | jq '.test_count')
        test_count_delta=$((head_test_count - base_test_count))
    fi

    # Research mode metrics
    local rca_available="false"
    local base_cyclomatic="0" head_cyclomatic="0" cyclomatic_delta="0"
    local base_cognitive="0" head_cognitive="0" cognitive_delta="0"
    local base_sloc="0" head_sloc="0" sloc_delta="0"
    local modules_available="false" base_modules="0" head_modules="0" modules_delta="0"
    local udeps_available="false" base_udeps="0" head_udeps="0" udeps_delta="0"

    if [[ "$MODE" == "research" ]]; then
        rca_available=$(echo "$head_json" | jq '.rust_code_analysis.available')
        base_cyclomatic=$(echo "$base_json" | jq '.rust_code_analysis.cyclomatic')
        head_cyclomatic=$(echo "$head_json" | jq '.rust_code_analysis.cyclomatic')
        cyclomatic_delta=$((head_cyclomatic - base_cyclomatic))
        base_cognitive=$(echo "$base_json" | jq '.rust_code_analysis.cognitive')
        head_cognitive=$(echo "$head_json" | jq '.rust_code_analysis.cognitive')
        cognitive_delta=$((head_cognitive - base_cognitive))
        base_sloc=$(echo "$base_json" | jq '.rust_code_analysis.sloc')
        head_sloc=$(echo "$head_json" | jq '.rust_code_analysis.sloc')
        sloc_delta=$((head_sloc - base_sloc))

        modules_available=$(echo "$head_json" | jq '.cargo_modules.available')
        base_modules=$(echo "$base_json" | jq '.cargo_modules.module_count')
        head_modules=$(echo "$head_json" | jq '.cargo_modules.module_count')
        modules_delta=$((head_modules - base_modules))

        udeps_available=$(echo "$head_json" | jq '.cargo_udeps.available')
        base_udeps=$(echo "$base_json" | jq '.cargo_udeps.unused_count')
        head_udeps=$(echo "$head_json" | jq '.cargo_udeps.unused_count')
        udeps_delta=$((head_udeps - base_udeps))
    fi

    # Format clippy status
    local head_clippy_status fmt_status test_status
    head_clippy_status=$(echo "$head_json" | jq -r '.clippy.status')
    fmt_status="pass"
    if [[ "$base_fmt" == "fail" ]] || [[ "$head_fmt" == "fail" ]]; then
        fmt_status="fail"
    fi
    test_status=$(echo "$head_json" | jq -r '.test.status')

    # Generate YAML
    cat <<EOF
# Telemetry Report
# Generated: $timestamp

pr: ${PR_NUMBER:-null}
base_sha: "$BASE_SHA"
head_sha: "$HEAD_SHA"
mode: $MODE
analyzed_at: "$timestamp"

tools:
  cargo_fmt:
    status: $fmt_status
    base: $base_fmt
    head: $head_fmt

  cargo_clippy:
    status: $head_clippy_status
    base_warnings: $base_clippy_warnings
    head_warnings: $head_clippy_warnings
    delta: $clippy_delta

  cargo_test:
    status: $test_status
    base_passed: $base_test_passed
    head_passed: $head_test_passed
    base_failed: $base_test_failed
    head_failed: $head_test_failed

  cargo_audit:
    available: $audit_available
    base_advisories: $base_audit
    head_advisories: $head_audit
    delta: $audit_delta

  shellcheck:
    status: $shellcheck_status
    base_issues: $shellcheck_base
    head_issues: $shellcheck_head
    delta: $shellcheck_delta

  actionlint:
    status: $actionlint_status
    base_issues: $actionlint_base
    head_issues: $actionlint_head
    delta: $actionlint_delta
EOF

    # Full mode sections
    if [[ "$MODE" == "full" ]] || [[ "$MODE" == "research" ]]; then
        cat <<EOF

  semver_checks:
    available: $semver_available
    breaking_changes: $semver_breaking

  geiger:
    available: $geiger_available
    base_unsafe: $base_unsafe
    head_unsafe: $head_unsafe
    delta: $geiger_delta

  typos:
    available: $typos_available
    base_count: $base_typos
    head_count: $head_typos
    delta: $typos_delta

  test_count:
    base: $base_test_count
    head: $head_test_count
    delta: $test_count_delta

  dependency_delta:
EOF
        # Format dependency arrays
        echo "    added:"
        if [[ -z "$deps_added" ]]; then
            echo "      []"
        else
            echo "$deps_added" | tr '|' '\n' | grep -v '^$' | sed 's/^/      - /'
        fi

        echo "    removed:"
        if [[ -z "$deps_removed" ]]; then
            echo "      []"
        else
            echo "$deps_removed" | tr '|' '\n' | grep -v '^$' | sed 's/^/      - /'
        fi

        echo "    updated:"
        if [[ -z "$deps_updated" ]]; then
            echo "      []"
        else
            echo "$deps_updated" | tr ' ' '\n' | grep -v '^$' | sed 's/^/      - /'
        fi
    fi

    # Research mode sections
    if [[ "$MODE" == "research" ]]; then
        cat <<EOF

  rust_code_analysis:
    available: $rca_available
    cyclomatic_complexity:
      base: $base_cyclomatic
      head: $head_cyclomatic
      delta: $cyclomatic_delta
    cognitive_complexity:
      base: $base_cognitive
      head: $head_cognitive
      delta: $cognitive_delta
    sloc:
      base: $base_sloc
      head: $head_sloc
      delta: $sloc_delta

  cargo_modules:
    available: $modules_available
    base_count: $base_modules
    head_count: $head_modules
    delta: $modules_delta

  cargo_udeps:
    available: $udeps_available
    base_unused: $base_udeps
    head_unused: $head_udeps
    delta: $udeps_delta
EOF
    fi

    # Summary
    cat <<EOF

summary:
  verdict: $verdict_result
  regressions:
EOF
    if [[ -z "$regressions" ]]; then
        echo "    []"
    else
        echo "$regressions" | tr ' ' '\n' | grep -v '^$' | sed 's/^/    - /'
    fi

    echo "  improvements:"
    if [[ -z "$improvements" ]]; then
        echo "    []"
    else
        echo "$improvements" | tr ' ' '\n' | grep -v '^$' | sed 's/^/    - /'
    fi
}

# =============================================================================
# Main Execution
# =============================================================================

main() {
    log_progress "Starting telemetry collection..."
    log_info "Mode: $MODE"

    # Detect tools
    detect_tools

    # Setup worktrees
    setup_worktrees

    # Create output directory
    mkdir -p "$OUTPUT_DIR"

    # Check for incremental run (skip if already computed)
    local report_file="${OUTPUT_DIR}/telemetry-${BASE_SHA:0:8}-${HEAD_SHA:0:8}-${MODE}.yaml"
    if [[ -f "$report_file" ]]; then
        log_info "Report already exists: $report_file"
        log_info "Use --output-dir to specify a different location or remove existing file"
        cat "$report_file"
        exit 0
    fi

    # Collect metrics for base
    log_progress "Analyzing base commit (${BASE_SHA:0:8})..."
    BASE_JSON=$(collect_metrics "$BASE_DIR" "base")
    echo "$BASE_JSON" > "${OUTPUT_DIR}/base-metrics.json"
    log_success "Base metrics collected"

    # Collect metrics for head
    log_progress "Analyzing head commit (${HEAD_SHA:0:8})..."
    HEAD_JSON=$(collect_metrics "$HEAD_DIR" "head")
    echo "$HEAD_JSON" > "${OUTPUT_DIR}/head-metrics.json"
    log_success "Head metrics collected"

    # Collect shellcheck/actionlint results (always-on, runs in both worktrees)
    log_progress "Analyzing scripts and workflows..."
    SHELLCHECK_RESULT=$(run_shellcheck "$BASE_DIR" "$HEAD_DIR")
    ACTIONLINT_RESULT=$(run_actionlint "$BASE_DIR" "$HEAD_DIR")

    # Dependency analysis (full mode only)
    DEP_DELTA=""
    if [[ "$MODE" == "full" ]] || [[ "$MODE" == "research" ]]; then
        DEP_DELTA=$(analyze_dependencies "$BASE_DIR" "$HEAD_DIR")
    fi

    # Generate YAML output to stdout
    log_progress "Generating report..."
    YAML_OUTPUT=$(generate_yaml_output "$BASE_JSON" "$HEAD_JSON" "$DEP_DELTA" "$SHELLCHECK_RESULT" "$ACTIONLINT_RESULT")
    echo "$YAML_OUTPUT"

    # Save YAML to file
    echo "$YAML_OUTPUT" > "$report_file"

    log_success "Telemetry collection complete"
    log_info "Report saved to: $report_file"
    log_info "Artifacts saved to: ${OUTPUT_DIR}/"
    log_info "  - base-metrics.json"
    log_info "  - head-metrics.json"
    log_info "  - telemetry-*.yaml"
}

main
