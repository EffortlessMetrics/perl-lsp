#!/bin/bash
# Benchmark runner with structured JSON output
#
# Usage:
#   ./run-benchmarks.sh                    # Run all, output to stdout
#   ./run-benchmarks.sh --output out.json  # Save to file
#   ./run-benchmarks.sh --quick            # Quick smoke test
#   ./run-benchmarks.sh --category parser  # Run specific category

set -euo pipefail

# Script directory
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"

# Defaults
OUTPUT_FILE=""
QUICK_MODE=false
CATEGORY=""
VERBOSE=false

# Parse arguments
while [[ $# -gt 0 ]]; do
    case $1 in
        --output|-o)
            OUTPUT_FILE="$2"
            shift 2
            ;;
        --quick|-q)
            QUICK_MODE=true
            shift
            ;;
        --category|-c)
            CATEGORY="$2"
            shift 2
            ;;
        --verbose|-v)
            VERBOSE=true
            shift
            ;;
        --help|-h)
            echo "Usage: $0 [OPTIONS]"
            echo ""
            echo "Options:"
            echo "  --output, -o FILE    Save results to JSON file"
            echo "  --quick, -q          Run quick smoke benchmarks"
            echo "  --category, -c CAT   Run specific category (parser, lexer, lsp, index)"
            echo "  --verbose, -v        Show detailed output"
            echo "  --help, -h           Show this help"
            exit 0
            ;;
        *)
            echo "Unknown option: $1"
            exit 1
            ;;
    esac
done

cd "$REPO_ROOT"

# Get environment info
GIT_SHA=$(git rev-parse --short HEAD 2>/dev/null || echo "unknown")
GIT_DIRTY=$(git diff --quiet 2>/dev/null && echo "false" || echo "true")
RUST_VERSION=$(rustc --version | cut -d' ' -f2)
OS_NAME=$(uname -s)
TIMESTAMP=$(date -u +"%Y-%m-%dT%H:%M:%SZ")

# Criterion args
CRITERION_ARGS="--locked"
if $QUICK_MODE; then
    CRITERION_ARGS="$CRITERION_ARGS -- --warm-up-time 1 --measurement-time 2 --sample-size 20"
fi

log() {
    if $VERBOSE; then
        echo "[$(date +%H:%M:%S)] $*" >&2
    fi
}

# Function to run benchmarks and extract results
run_criterion_bench() {
    local crate=$1
    local bench=$2
    local category=$3

    log "Running $crate::$bench..."

    # Create temp file for output
    local temp_output
    temp_output=$(mktemp)

    # Run benchmark, capturing output
    if cargo bench -p "$crate" --bench "$bench" $CRITERION_ARGS 2>&1 | tee "$temp_output" > /dev/null; then
        # Parse criterion output for timing
        # Example: "parse_simple_script  time:   [45.123 us 45.234 us 45.345 us]"
        while IFS= read -r line; do
            if [[ $line =~ ([a-z_]+)[[:space:]]+time:[[:space:]]+\[([0-9.]+)[[:space:]]+(ns|us|ms|s)[[:space:]]+([0-9.]+)[[:space:]]+(ns|us|ms|s)[[:space:]]+([0-9.]+)[[:space:]]+(ns|us|ms|s)\] ]]; then
                local bench_name="${BASH_REMATCH[1]}"
                local low="${BASH_REMATCH[2]}"
                local mean="${BASH_REMATCH[4]}"
                local unit="${BASH_REMATCH[5]}"
                local high="${BASH_REMATCH[6]}"

                # Convert to nanoseconds
                local multiplier=1
                case $unit in
                    us) multiplier=1000 ;;
                    ms) multiplier=1000000 ;;
                    s)  multiplier=1000000000 ;;
                esac

                local mean_ns
                mean_ns=$(echo "$mean * $multiplier" | bc | cut -d'.' -f1)
                local low_ns
                low_ns=$(echo "$low * $multiplier" | bc | cut -d'.' -f1)
                local high_ns
                high_ns=$(echo "$high * $multiplier" | bc | cut -d'.' -f1)

                echo "      \"$bench_name\": {"
                echo "        \"mean_ns\": $mean_ns,"
                echo "        \"low_ns\": $low_ns,"
                echo "        \"high_ns\": $high_ns,"
                echo "        \"unit\": \"$unit\","
                echo "        \"display\": \"$mean $unit\""
                echo "      },"
            fi
        done < "$temp_output"
    fi

    rm -f "$temp_output"
}

# Start JSON output
json_output() {
    echo "{"
    echo "  \"version\": \"0.9.0\","
    echo "  \"timestamp\": \"$TIMESTAMP\","
    echo "  \"git_sha\": \"$GIT_SHA\","
    echo "  \"git_dirty\": $GIT_DIRTY,"
    echo "  \"environment\": {"
    echo "    \"os\": \"$OS_NAME\","
    echo "    \"rust_version\": \"$RUST_VERSION\","
    echo "    \"quick_mode\": $QUICK_MODE"
    echo "  },"
    echo "  \"results\": {"

    # Parser benchmarks
    if [[ -z "$CATEGORY" || "$CATEGORY" == "parser" ]]; then
        echo "    \"parser\": {"
        run_criterion_bench "perl-parser" "parser_benchmark" "parser"
        # Remove trailing comma from last entry
        echo "      \"_category\": \"parser\""
        echo "    },"
    fi

    # Lexer benchmarks
    if [[ -z "$CATEGORY" || "$CATEGORY" == "lexer" ]]; then
        echo "    \"lexer\": {"
        run_criterion_bench "perl-lexer" "lexer_benchmarks" "lexer"
        echo "      \"_category\": \"lexer\""
        echo "    },"
    fi

    # LSP benchmarks
    if [[ -z "$CATEGORY" || "$CATEGORY" == "lsp" ]]; then
        echo "    \"lsp\": {"
        run_criterion_bench "perl-lsp" "rope_performance_benchmark" "lsp"
        echo "      \"_category\": \"lsp\""
        echo "    },"
    fi

    # Workspace index benchmarks
    if [[ -z "$CATEGORY" || "$CATEGORY" == "index" ]]; then
        echo "    \"index\": {"
        run_criterion_bench "perl-workspace-index" "workspace_index_benchmark" "index"
        echo "      \"_category\": \"index\""
        echo "    }"
    else
        # Remove trailing comma if index was skipped
        echo "    \"_done\": true"
    fi

    echo "  }"
    echo "}"
}

# Run and output
if [[ -n "$OUTPUT_FILE" ]]; then
    log "Saving results to $OUTPUT_FILE"
    json_output > "$OUTPUT_FILE"
    echo "Results saved to $OUTPUT_FILE"
else
    json_output
fi
