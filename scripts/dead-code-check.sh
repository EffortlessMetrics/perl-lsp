#!/usr/bin/env bash
# Dead code detection for perl-lsp workspace
# Combines cargo-udeps (unused dependencies) and clippy dead_code lint

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
cd "$REPO_ROOT"

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Configuration
BASELINE_FILE=".ci/dead-code-baseline.yaml"
MODE="${1:-check}" # check, baseline, or report
STRICT="${DEAD_CODE_STRICT:-false}"

log_info() {
    echo -e "${BLUE}[INFO]${NC} $*"
}

log_success() {
    echo -e "${GREEN}[SUCCESS]${NC} $*"
}

log_warn() {
    echo -e "${YELLOW}[WARN]${NC} $*"
}

log_error() {
    echo -e "${RED}[ERROR]${NC} $*"
}

# Check if required tools are available
check_tools() {
    local missing_tools=()

    if ! command -v cargo >/dev/null 2>&1; then
        missing_tools+=("cargo")
    fi

    # cargo-udeps requires nightly
    if ! rustup toolchain list | grep -q nightly; then
        log_warn "Nightly toolchain not installed (required for cargo-udeps)"
        log_info "Install with: rustup toolchain install nightly"
        return 1
    fi

    if [ "${#missing_tools[@]}" -gt 0 ]; then
        log_error "Missing required tools: ${missing_tools[*]}"
        log_info "Install cargo: https://rustup.rs"
        return 1
    fi

    return 0
}

# Install cargo-udeps if not present
install_udeps() {
    if ! cargo +nightly udeps --version >/dev/null 2>&1; then
        log_info "Installing cargo-udeps..."
        cargo install cargo-udeps --locked
    fi
}

# Run cargo-machete to detect unused dependencies (fast)
check_unused_deps_machete() {
    log_info "Checking for unused dependencies with cargo-machete..."

    local output_file="target/dead-code/machete-output.txt"
    mkdir -p "$(dirname "$output_file")"

    if cargo machete 2>&1 | tee "$output_file"; then
        if grep -q "unused dependencies" "$output_file"; then
            local unused_count
            unused_count=$(grep -c "Cargo.toml:" "$output_file") || unused_count=0
            log_warn "Found $unused_count crates with unused dependencies"
            return 0
        else
            log_success "No unused dependencies detected by machete"
            return 0
        fi
    else
        log_error "cargo-machete failed to run"
        return 1
    fi
}

# Run cargo-udeps to detect unused dependencies (deep)
check_unused_deps_udeps() {
    log_info "Checking for unused dependencies with cargo-udeps..."

    local output_file="target/dead-code/udeps-output.txt"
    mkdir -p "$(dirname "$output_file")"

    # Run cargo-udeps on all workspace members
    # Note: cargo-udeps requires nightly Rust
    if cargo +nightly udeps --workspace --all-targets --locked 2>&1 | tee "$output_file"; then
        local unused_count
        unused_count=$(grep -c "unused" "$output_file") || unused_count=0

        if [ "$unused_count" -eq 0 ]; then
            log_success "No unused dependencies detected"
            return 0
        else
            log_warn "Found $unused_count unused dependency warnings"
            if [ "$MODE" = "check" ] && [ "$STRICT" = "true" ]; then
                return 1
            fi
            return 0
        fi
    else
        log_error "cargo-udeps failed to run"
        return 1
    fi
}

# Run clippy with dead_code lint
check_dead_code_clippy() {
    log_info "Checking for dead code with clippy..."

    local output_file="target/dead-code/clippy-dead-code.txt"
    mkdir -p "$(dirname "$output_file")"

    # Run clippy with dead_code warnings
    # We only check libraries and bins, not tests
    if cargo clippy --workspace --lib --bins --locked \
        -- -W dead_code -W unused_imports -W unused_variables \
        2>&1 | tee "$output_file"; then

        local dead_code_count
        dead_code_count=$(grep -c "dead_code" "$output_file") || dead_code_count=0

        if [ "$dead_code_count" -eq 0 ]; then
            log_success "No dead code detected by clippy"
            return 0
        else
            log_warn "Found $dead_code_count dead code warnings"
            if [ "$MODE" = "check" ] && [ "$STRICT" = "true" ]; then
                return 1
            fi
            return 0
        fi
    else
        # Clippy may exit with error if there are warnings, that's expected
        local dead_code_count
        dead_code_count=$(grep -c "dead_code" "$output_file") || dead_code_count=0

        if [ "$dead_code_count" -gt 0 ]; then
            log_warn "Found $dead_code_count dead code warnings"
        fi

        # Don't fail on clippy warnings in non-strict mode
        if [ "$STRICT" = "false" ]; then
            return 0
        fi
        return 1
    fi
}

# Generate baseline file
generate_baseline() {
    log_info "Generating dead code baseline..."

    mkdir -p "$(dirname "$BASELINE_FILE")"

    # Run checks and capture output
    local udeps_output="target/dead-code/udeps-output.txt"
    local clippy_output="target/dead-code/clippy-dead-code.txt"

    mkdir -p target/dead-code

    # Capture udeps results (don't fail on errors)
    cargo +nightly udeps --workspace --all-targets --locked > "$udeps_output" 2>&1 || true

    # Capture clippy results (don't fail on errors)
    cargo clippy --workspace --lib --bins --locked \
        -- -W dead_code -W unused_imports -W unused_variables \
        > "$clippy_output" 2>&1 || true

    # Count issues
    local unused_deps_count
    unused_deps_count=$(grep -c "unused" "$udeps_output") || unused_deps_count=0

    local dead_code_count
    dead_code_count=$(grep -c "dead_code" "$clippy_output") || dead_code_count=0

    local unused_imports_count
    unused_imports_count=$(grep -c "unused_imports" "$clippy_output") || unused_imports_count=0

    local unused_vars_count
    unused_vars_count=$(grep -c "unused_variables" "$clippy_output") || unused_vars_count=0

    # Generate YAML baseline
    cat > "$BASELINE_FILE" <<EOF
# Dead Code Detection Baseline
# Generated: $(date -u +"%Y-%m-%d %H:%M:%S UTC")
#
# This file tracks the baseline for dead code detection.
# When new dead code is introduced, the checks will fail.
#
# To update this baseline: just dead-code-baseline

schema_version: 1
last_updated: "$(date -u +"%Y-%m-%d")"

# Thresholds (fail if exceeded)
thresholds:
  # Maximum allowed unused dependencies
  max_unused_dependencies: 5

  # Maximum allowed dead code items (functions, types, etc.)
  max_dead_code_items: 10

  # Maximum allowed unused imports
  max_unused_imports: 20

  # Maximum allowed unused variables
  max_unused_variables: 10

# Current baseline counts
baseline:
  unused_dependencies: $unused_deps_count
  dead_code_items: $dead_code_count
  unused_imports: $unused_imports_count
  unused_variables: $unused_vars_count

# Allowed exceptions (items that are intentionally unused)
allowed_exceptions:
  # Example: functions that are part of public API but not used internally
  # - crate: perl-parser
  #   type: function
  #   name: parse_legacy_syntax
  #   reason: "Public API for backward compatibility"

  # Example: dependencies used only in specific build configurations
  # - crate: perl-lsp
  #   dependency: tokio
  #   reason: "Used in async runtime feature"

# Known issues to be addressed
known_issues:
  # Track specific dead code items that need cleanup
  # - crate: perl-parser-core
  #   type: dead_code
  #   item: legacy_parser_function
  #   issue: "#XXX"
  #   notes: "Remove after migration to v3 parser"

# Policy
policy:
  # Enforcement level: strict, warn, or disabled
  enforcement: warn

  # Auto-update baseline on PR (requires manual approval)
  auto_update_baseline: false

  # Fail CI if baseline is exceeded
  fail_on_baseline_exceeded: true

  # Warn if approaching threshold (80% of max)
  warn_threshold_percent: 80

# Maintenance
maintenance:
  # Review dead code baseline every N days
  review_interval_days: 30

  # Next scheduled review
  next_review: "$(date -u -d '+30 days' +"%Y-%m-%d" 2>/dev/null || date -v+30d -u +"%Y-%m-%d")"
EOF

    log_success "Baseline saved to $BASELINE_FILE"
    echo ""
    log_info "Current counts:"
    echo "  Unused dependencies: $unused_deps_count"
    echo "  Dead code items:     $dead_code_count"
    echo "  Unused imports:      $unused_imports_count"
    echo "  Unused variables:    $unused_vars_count"
}

# Compare current state against baseline
check_against_baseline() {
    if [ ! -f "$BASELINE_FILE" ]; then
        log_warn "No baseline file found at $BASELINE_FILE"
        log_info "Generate baseline with: just dead-code-baseline"
        return 0
    fi

    log_info "Checking against baseline..."

    # Run current checks
    local udeps_output="target/dead-code/udeps-output.txt"
    local clippy_output="target/dead-code/clippy-dead-code.txt"

    mkdir -p target/dead-code

    cargo +nightly udeps --workspace --all-targets --locked > "$udeps_output" 2>&1 || true
    cargo clippy --workspace --lib --bins --locked \
        -- -W dead_code -W unused_imports -W unused_variables \
        > "$clippy_output" 2>&1 || true

    # Count current issues
    local current_unused_deps
    current_unused_deps=$(grep -c "unused" "$udeps_output") || current_unused_deps=0

    local current_dead_code
    current_dead_code=$(grep -c "dead_code" "$clippy_output") || current_dead_code=0

    # Extract baseline values
    local baseline_unused_deps
    baseline_unused_deps=$(grep "unused_dependencies:" "$BASELINE_FILE" | awk '{print $2}')

    local baseline_dead_code
    baseline_dead_code=$(grep "dead_code_items:" "$BASELINE_FILE" | awk '{print $2}')

    # Extract thresholds
    local max_unused_deps
    max_unused_deps=$(grep "max_unused_dependencies:" "$BASELINE_FILE" | awk '{print $2}')

    local max_dead_code
    max_dead_code=$(grep "max_dead_code_items:" "$BASELINE_FILE" | awk '{print $2}')

    log_info "Comparison:"
    echo "  Unused dependencies: $current_unused_deps (baseline: $baseline_unused_deps, max: $max_unused_deps)"
    echo "  Dead code items:     $current_dead_code (baseline: $baseline_dead_code, max: $max_dead_code)"

    local failed=false

    # Check if current exceeds thresholds
    if [ "$current_unused_deps" -gt "$max_unused_deps" ]; then
        log_error "Unused dependencies ($current_unused_deps) exceeds threshold ($max_unused_deps)"
        failed=true
    fi

    if [ "$current_dead_code" -gt "$max_dead_code" ]; then
        log_error "Dead code items ($current_dead_code) exceeds threshold ($max_dead_code)"
        failed=true
    fi

    # Check if current exceeds baseline (regression)
    if [ "$current_unused_deps" -gt "$baseline_unused_deps" ]; then
        log_warn "Unused dependencies increased from $baseline_unused_deps to $current_unused_deps"
        if [ "$STRICT" = "true" ]; then
            failed=true
        fi
    fi

    if [ "$current_dead_code" -gt "$baseline_dead_code" ]; then
        log_warn "Dead code increased from $baseline_dead_code to $current_dead_code"
        if [ "$STRICT" = "true" ]; then
            failed=true
        fi
    fi

    if [ "$failed" = "true" ]; then
        log_error "Dead code checks failed"
        return 1
    fi

    log_success "Dead code checks passed"
    return 0
}

# Generate JSON report for CI integration
generate_report() {
    log_info "Generating JSON report..."

    local report_file="target/dead-code/report.json"
    mkdir -p "$(dirname "$report_file")"

    # Run checks
    local udeps_output="target/dead-code/udeps-output.txt"
    local clippy_output="target/dead-code/clippy-dead-code.txt"

    cargo +nightly udeps --workspace --all-targets --locked > "$udeps_output" 2>&1 || true
    cargo clippy --workspace --lib --bins --locked \
        -- -W dead_code -W unused_imports -W unused_variables \
        > "$clippy_output" 2>&1 || true

    local unused_deps
    unused_deps=$(grep -c "unused" "$udeps_output") || unused_deps=0

    local dead_code
    dead_code=$(grep -c "dead_code" "$clippy_output") || dead_code=0

    local unused_imports
    unused_imports=$(grep -c "unused_imports" "$clippy_output") || unused_imports=0

    local unused_vars
    unused_vars=$(grep -c "unused_variables" "$clippy_output") || unused_vars=0

    # Generate JSON
    cat > "$report_file" <<EOF
{
  "schema_version": 1,
  "timestamp": "$(date -u +"%Y-%m-%dT%H:%M:%SZ")",
  "results": {
    "unused_dependencies": $unused_deps,
    "dead_code_items": $dead_code,
    "unused_imports": $unused_imports,
    "unused_variables": $unused_vars,
    "total_issues": $((unused_deps + dead_code + unused_imports + unused_vars))
  },
  "details": {
    "udeps_output": "$udeps_output",
    "clippy_output": "$clippy_output"
  }
}
EOF

    log_success "Report saved to $report_file"
    cat "$report_file"
}

# Main execution
main() {
    log_info "Dead Code Detection (mode: $MODE)"
    echo ""

    if ! check_tools; then
        log_error "Prerequisites not met"
        exit 1
    fi

    case "$MODE" in
        baseline)
            install_udeps
            generate_baseline
            ;;
        report)
            install_udeps
            generate_report
            ;;
        check)
            if [ -f "$BASELINE_FILE" ]; then
                check_against_baseline
            else
                log_info "No baseline file, running basic checks..."
                check_unused_deps_machete
                check_dead_code_clippy
            fi
            ;;
        *)
            log_error "Unknown mode: $MODE"
            log_info "Usage: $0 [check|baseline|report]"
            exit 1
            ;;
    esac
}

main "$@"
