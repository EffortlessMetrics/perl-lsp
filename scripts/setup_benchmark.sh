#!/bin/bash
#
# Benchmark Framework Setup Script
#
# This script sets up all dependencies and configuration needed for the
# tree-sitter-perl benchmark framework.

set -euo pipefail

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[0;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Logging functions
log_info() {
    echo -e "${BLUE}[INFO]${NC} $1"
}

log_success() {
    echo -e "${GREEN}[SUCCESS]${NC} $1"
}

log_warning() {
    echo -e "${YELLOW}[WARNING]${NC} $1"
}

log_error() {
    echo -e "${RED}[ERROR]${NC} $1"
}

# Check if command exists
command_exists() {
    command -v "$1" >/dev/null 2>&1
}

# Project root directory
PROJECT_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$PROJECT_ROOT"

log_info "Setting up tree-sitter-perl benchmark framework"
log_info "Project root: $PROJECT_ROOT"

# Check required system dependencies
log_info "Checking system dependencies..."

MISSING_DEPS=()

if ! command_exists cargo; then
    MISSING_DEPS+=("cargo (Rust toolchain)")
fi

if ! command_exists node; then
    MISSING_DEPS+=("node (Node.js)")
fi

if ! command_exists python3; then
    MISSING_DEPS+=("python3")
fi

if ! command_exists npm; then
    MISSING_DEPS+=("npm (Node.js package manager)")
fi

if [ ${#MISSING_DEPS[@]} -ne 0 ]; then
    log_error "Missing required dependencies:"
    for dep in "${MISSING_DEPS[@]}"; do
        echo "  - $dep"
    done
    echo
    log_info "Please install the missing dependencies and run this script again."
    echo
    echo "Installation guides:"
    echo "  Rust: https://rustup.rs/"
    echo "  Node.js: https://nodejs.org/"
    echo "  Python: https://python.org/"
    exit 1
fi

log_success "All system dependencies found"

# Check Rust toolchain version
log_info "Checking Rust toolchain..."
RUST_VERSION=$(cargo --version | cut -d' ' -f2)
log_info "Rust version: $RUST_VERSION"

# Ensure we have the required Rust version (1.92+)
REQUIRED_RUST_VERSION="1.92"
if ! cargo --version | grep -q "1.9[2-9]" && ! cargo --version | grep -q "1.[1-9][0-9][0-9]"; then
    log_warning "Rust version may be too old. Required: $REQUIRED_RUST_VERSION+"
    log_warning "Current version: $RUST_VERSION"
    log_info "Consider updating with: rustup update"
fi

# Build Rust benchmark components
log_info "Building Rust benchmark components..."

# Build main benchmark runner
if cargo build -p tree-sitter-perl --bin ts_benchmark_parsers --features pure-rust --release; then
    log_success "Built Rust benchmark runner"
else
    log_error "Failed to build Rust benchmark runner"
    exit 1
fi

# Build Pest benchmark runner (legacy)
if cargo build -p perl-parser-pest --bin benchmark_parsers --release; then
    log_success "Built Pest benchmark runner"
else
    log_warning "Failed to build Pest benchmark runner (legacy component)"
fi

# Set up Node.js dependencies for C benchmarking
log_info "Setting up Node.js dependencies for C benchmarking..."

cd tree-sitter-perl

if [ -f package.json ]; then
    if npm install; then
        log_success "Installed Node.js dependencies"
    else
        log_error "Failed to install Node.js dependencies"
        exit 1
    fi
else
    log_warning "No package.json found in tree-sitter-perl directory"
    log_info "C benchmarking may not be available"
fi

cd "$PROJECT_ROOT"

# Set up Python dependencies
log_info "Setting up Python dependencies..."

if [ -f requirements.txt ]; then
    log_info "Installing Python dependencies (optional)..."
    
    # Try to create virtual environment
    if python3 -m venv venv 2>/dev/null; then
        log_success "Created Python virtual environment"
        source venv/bin/activate
        
        if pip install -r requirements.txt; then
            log_success "Installed Python dependencies"
            log_info "Virtual environment created at: $PROJECT_ROOT/venv"
            log_info "Activate with: source venv/bin/activate"
        else
            log_warning "Failed to install some Python dependencies"
            log_info "The comparison script will still work with core functionality"
        fi
    else
        log_info "Installing Python dependencies globally..."
        if pip3 install -r requirements.txt; then
            log_success "Installed Python dependencies"
        else
            log_warning "Failed to install some Python dependencies"
            log_info "The comparison script will still work with core functionality"
        fi
    fi
else
    log_info "No requirements.txt found, skipping Python dependencies"
fi

# Create default configuration files
log_info "Creating default configuration files..."

# Create benchmark configuration
BENCHMARK_CONFIG="benchmark_config.json"
if [ ! -f "$BENCHMARK_CONFIG" ]; then
    cat > "$BENCHMARK_CONFIG" << 'EOF'
{
  "iterations": 100,
  "warmup_iterations": 10,
  "test_files": [
    "test/benchmark_simple.pl",
    "test/corpus"
  ],
  "output_path": "benchmark_results.json",
  "detailed_stats": true,
  "memory_tracking": false
}
EOF
    log_success "Created benchmark configuration: $BENCHMARK_CONFIG"
else
    log_info "Benchmark configuration already exists: $BENCHMARK_CONFIG"
fi

# Create comparison configuration
COMPARISON_CONFIG="comparison_config.json"
if python3 scripts/generate_comparison.py --create-config "$COMPARISON_CONFIG" >/dev/null 2>&1; then
    log_success "Created comparison configuration: $COMPARISON_CONFIG"
else
    log_warning "Failed to create comparison configuration"
fi

# Verify installation
log_info "Verifying installation..."

# Test Rust benchmark runner
log_info "Testing Rust benchmark runner..."
if timeout 30 cargo run -p tree-sitter-perl --bin ts_benchmark_parsers --features pure-rust --release 2>/dev/null; then
    log_success "Rust benchmark runner works"
else
    log_warning "Rust benchmark runner test failed or timed out"
fi

# Test Python comparison script
log_info "Testing Python comparison script..."
if python3 scripts/generate_comparison.py --help >/dev/null 2>&1; then
    log_success "Python comparison script works"
else
    log_error "Python comparison script test failed"
fi

# Test Node.js benchmark script (if available)
if [ -f tree-sitter-perl/test/benchmark.js ]; then
    log_info "Testing Node.js benchmark script..."
    cd tree-sitter-perl
    if TEST_CODE="print 'hello';" ITERATIONS=1 timeout 10 node test/benchmark.js >/dev/null 2>&1; then
        log_success "Node.js benchmark script works"
    else
        log_warning "Node.js benchmark script test failed or timed out"
    fi
    cd "$PROJECT_ROOT"
fi

# Display summary
echo
log_success "Benchmark framework setup complete!"
echo
echo "Available commands:"
echo "  cargo xtask bench                    # Run complete benchmark suite"
echo "  cargo run --bin ts_benchmark_parsers # Run Rust benchmarks only"
echo "  python3 scripts/generate_comparison.py --help  # Show comparison options"
echo
echo "Configuration files:"
echo "  benchmark_config.json               # Rust benchmark settings"
echo "  comparison_config.json              # Comparison thresholds"
echo
echo "Documentation:"
echo "  docs/BENCHMARK_FRAMEWORK.md         # Comprehensive guide"
echo
log_info "Run 'cargo xtask bench' to start benchmarking!"