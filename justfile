# Justfile for perl-lsp development and CI workflows
# Usage: just <command>
# Install just: cargo install just

# Default recipe (show available commands)
default:
    @just --list

# ============================================================================
# CI Validation Commands (Issue #211)
# ============================================================================

# Fast merge gate (~2-5 min) - REQUIRED for all merges
ci-gate:
    @echo "🚪 Running fast merge gate..."
    @just ci-format
    @just ci-clippy-lib
    @just ci-test-lib
    @just ci-policy
    @just ci-lsp-def
    @echo "✅ Merge gate passed!"

# Full CI pipeline (~10-20 min) - RECOMMENDED for large changes
ci-full:
    @echo "🚀 Running full CI pipeline..."
    @just ci-format
    @just ci-clippy
    @just ci-test-core
    @just ci-test-lsp
    @just ci-docs || true
    @echo "✅ Full CI passed!"

# Legacy alias (deprecated, use ci-full)
ci-local:
    @echo "⚠️  'ci-local' is deprecated, use 'ci-full' instead"
    @just ci-full

# Format check (fast fail)
ci-format:
    @echo "📝 Checking code formatting..."
    cargo fmt --check --all
    @echo "✅ Format check passed"

# Clippy lint (catches common issues, allow missing_docs during systematic resolution)
ci-clippy:
    @echo "🔍 Running clippy (all targets)..."
    cargo clippy --workspace --all-targets -- -D warnings -A missing_docs
    @echo "✅ Clippy passed"

# Clippy libraries only (fast, for merge gate)
ci-clippy-lib:
    @echo "🔍 Running clippy (libraries only)..."
    cargo clippy --workspace --lib --locked -- -D warnings -A missing_docs
    @echo "✅ Clippy (lib) passed"

# Core tests (fast, essential)
ci-test-core:
    @echo "🧪 Running core tests..."
    cargo test --workspace --lib --bins
    @echo "✅ Core tests passed"

# Library tests only (fastest, for merge gate)
ci-test-lib:
    @echo "🧪 Running library tests..."
    cargo test --workspace --lib --locked
    @echo "✅ Library tests passed"

# LSP integration tests (with adaptive threading)
ci-test-lsp:
    @echo "🔌 Running LSP integration tests..."
    RUST_TEST_THREADS=2 cargo test -p perl-lsp --test lsp_comprehensive_e2e_test -- --test-threads=2
    @echo "✅ LSP tests passed"

# LSP semantic definition tests (semantic-aware go-to-definition)
ci-lsp-def:
    @echo "🔎 Running LSP semantic definition tests..."
    RUSTC_WRAPPER="" RUST_TEST_THREADS=1 CARGO_BUILD_JOBS=1 \
        cargo test -p perl-lsp --test semantic_definition -- --test-threads=1
    @echo "✅ LSP semantic definition tests passed"

# Documentation build (no deps)
ci-docs:
    @echo "📚 Building documentation..."
    cargo doc -p perl-parser -p perl-lsp --no-deps
    @echo "✅ Docs build passed"

# Mutation testing (expensive, ~15-30 min)
ci-test-mutation:
    @echo "🧬 Running mutation tests..."
    cargo mutants --package perl-parser --timeout 300
    @echo "✅ Mutation tests passed"

# Cost estimation
ci-cost-estimate:
    @echo "💰 Estimating CI costs (essential jobs: ~$0.06-0.08 per PR)"
    @just ci-local

# ============================================================================
# Development Commands
# ============================================================================

# Build all workspace crates
build:
    cargo build --workspace

# Run all tests
test:
    cargo test --workspace

# Format code
fmt:
    cargo fmt --all

# Clean build artifacts
clean:
    cargo clean


# Policy enforcement checks
ci-policy:
    @echo "📋 Running policy checks..."
    @./.ci/scripts/check-from-raw.sh
    @echo "✅ Policy checks passed"
