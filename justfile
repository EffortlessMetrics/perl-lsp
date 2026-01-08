# Justfile for perl-lsp development and CI workflows
# Usage: just <command>
# Install just: cargo install just

# Default recipe (show available commands)
default:
    @just --list

# ============================================================================
# CI Validation Commands (Issue #211)
# ============================================================================

# MSRV: Rust 1.89 (for OpenAI Codex compatibility)
# The rust-toolchain.toml pins to 1.89.0, so standard commands use MSRV by default.
# Use these recipes to explicitly verify MSRV compliance:

# Fast merge gate on MSRV (~2-5 min) - proves 1.89 compatibility
ci-gate-msrv:
    @echo "🚪 Running fast merge gate on MSRV (Rust 1.89)..."
    @RUSTUP_TOOLCHAIN=1.89.0 just ci-gate

# Full CI on MSRV (~10-20 min) - proves 1.89 compatibility for releases
ci-full-msrv:
    @echo "🚀 Running full CI on MSRV (Rust 1.89)..."
    @RUSTUP_TOOLCHAIN=1.89.0 just ci-full

# Check for nested Cargo.lock files (footgun prevention)
ci-check-no-nested-lock:
    @echo "🔒 Checking for nested Cargo.lock files..."
    @if find . -name 'Cargo.lock' -type f 2>/dev/null | grep -v '^\./Cargo\.lock$' | grep -q .; then \
        echo "❌ ERROR: Nested Cargo.lock detected! Run gates from repo root only."; \
        find . -name 'Cargo.lock' -type f 2>/dev/null | grep -v '^\./Cargo\.lock$'; \
        exit 1; \
    fi
    @echo "✅ No nested lockfiles"

# Audit workflows for ungated expensive jobs
ci-workflow-audit:
    @python3 scripts/ci-audit-workflows.py

# Fast merge gate (~2-5 min) - REQUIRED for all merges
ci-gate:
    @echo "🚪 Running fast merge gate..."
    @just ci-workflow-audit
    @just ci-check-no-nested-lock
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

# Missing docs ratcheting check (Issue #197)
ci-docs-check:
    @echo "📝 Checking missing docs baseline..."
    @bash ci/check_missing_docs.sh
    @echo "✅ Missing docs check passed"

# Policy enforcement checks
ci-policy:
    @echo "📋 Running policy checks..."
    @./.ci/scripts/check-from-raw.sh
    @just status-check
    @just ci-docs-check
    @echo "✅ Policy checks passed"

# Update derived metrics in CURRENT_STATUS.md
status-update:
    @python3 scripts/update-current-status.py --write

# Verify CURRENT_STATUS.md derived metrics are up-to-date
status-check:
    @python3 scripts/update-current-status.py --check

# ============================================================================
# Corpus Audit Commands
# ============================================================================

# Run corpus audit for coverage analysis
corpus-audit:
    @echo "🔍 Running corpus audit..."
    @cd xtask && cargo run --no-default-features -- corpus-audit

# Run corpus audit in CI check mode (fails if issues found)
corpus-audit-check:
    @echo "🔍 Running corpus audit (CI check mode)..."
    @cd xtask && cargo run --no-default-features -- corpus-audit --check

# Run corpus audit with fresh report regeneration
corpus-audit-fresh:
    @echo "🔍 Running corpus audit (fresh mode)..."
    @cd xtask && cargo run --no-default-features -- corpus-audit --fresh

# ============================================================================
# GitHub Repository Management
# ============================================================================

# Ensure label taxonomy exists (idempotent, safe to rerun)
gh-labels:
    @echo "🏷️  Ensuring label taxonomy..."
    @bash scripts/gh/ensure-labels.sh
    @echo "✅ Labels ready"

# Show issues missing required taxonomy labels
gh-triage:
    @echo "🔍 Issues needing taxonomy labels..."
    @bash scripts/gh/issues-needing-triage.sh 500

# Backfill prefixed labels from legacy labels (dry run)
gh-backfill-dry:
    @echo "🔄 Dry run: showing labels to backfill..."
    @bash scripts/gh/backfill-prefixed-labels.sh

# Backfill prefixed labels from legacy labels (apply)
gh-backfill:
    @echo "🔄 Applying prefixed label backfill..."
    @bash scripts/gh/backfill-prefixed-labels.sh --apply

# ============================================================================
# Bug Tracking (BUG category ignored tests)
# ============================================================================

# Show current bug status
bugs:
    @echo "🐛 Bug Queue Status"
    @echo "==================="
    @VERBOSE=1 bash scripts/ignored-test-count.sh 2>&1 | sed -n '/=== bug/,/===/p' | head -30

# Wave A: COMPLETE - these were test brittleness issues, not parser bugs
bugs-wave-a:
    @echo "✅ Wave A: Complete (tests were brittle, not bugs)"
    @echo "   - test_word_boundary_qwerty_not_matched: fixed test expectations"
    @echo "   - test_comment_with_qw_in_it: fixed dynamic position calculation"

# Run all Wave B bug tests (substitution)
bugs-wave-b:
    @echo "🌊 Wave B: Substitution Operator Bugs"
    cargo test -p perl-parser --test substitution_operator_tests -- test_substitution_empty_replacement_balanced_delimiters --nocapture --ignored || true
    cargo test -p perl-parser --test substitution_ac_tests -- test_ac2_empty_replacement_balanced_delimiters --nocapture --ignored || true
    cargo test -p perl-parser --test substitution_operator_tests -- test_substitution_invalid_modifier_characters --nocapture --ignored || true
    cargo test -p perl-parser --test substitution_ac_tests -- test_ac2_invalid_flag_combinations --nocapture --ignored || true

# Run all Wave C bug tests (harder semantics)
bugs-wave-c:
    @echo "🌊 Wave C: Semantic Bugs"
    cargo test -p perl-parser --test substitution_ac_tests -- test_ac5_negative_malformed --nocapture --ignored || true
    cargo test -p perl-parser --test prop_whitespace_idempotence -- insertion_safe_is_consistent --nocapture --ignored || true
    cargo test -p perl-parser --test comprehensive_operator_precedence_test -- test_complex_precedence_combinations --nocapture --ignored || true
    cargo test -p perl-parser --test parser_regressions -- print_filehandle_then_variable_is_indirect --nocapture --ignored || true

# ============================================================================
# Health Scoreboard (keep yourself honest)
# ============================================================================

# Show codebase health metrics
health:
    @echo "📊 Codebase Health Scoreboard"
    @echo "=============================="
    @echo ""
    @echo "📝 Ignored Tests by Crate:"
    @echo "  perl-parser: $(grep -r '#\[ignore' crates/perl-parser/tests/ 2>/dev/null | wc -l || echo 0)"
    @echo "  perl-lsp:    $(grep -r '#\[ignore' crates/perl-lsp/tests/ 2>/dev/null | wc -l || echo 0)"
    @echo "  perl-lexer:  $(grep -r '#\[ignore' crates/perl-lexer/tests/ 2>/dev/null | wc -l || echo 0)"
    @echo "  perl-dap:    $(grep -r '#\[ignore' crates/perl-dap/tests/ 2>/dev/null | wc -l || echo 0)"
    @echo ""
    @echo "⚠️  Unwrap/Expect Count (potential panic sites):"
    @echo "  .unwrap():  $(grep -r '\.unwrap()' crates/*/src/ --include='*.rs' 2>/dev/null | wc -l || echo 0)"
    @echo "  .expect(:   $(grep -r '\.expect(' crates/*/src/ --include='*.rs' 2>/dev/null | wc -l || echo 0)"
    @echo ""
    @echo "🖨️  Debug Print Count (should use tracing):"
    @echo "  println!:   $(grep -r 'println!' crates/*/src/ --include='*.rs' 2>/dev/null | wc -l || echo 0)"
    @echo "  eprintln!:  $(grep -r 'eprintln!' crates/*/src/ --include='*.rs' 2>/dev/null | wc -l || echo 0)"
    @echo ""
    @echo "📦 Public Items in perl-parser (API surface):"
    @echo "  pub fn:     $(grep -r '^[[:space:]]*pub fn' crates/perl-parser/src/ --include='*.rs' 2>/dev/null | wc -l || echo 0)"
    @echo "  pub struct: $(grep -r '^[[:space:]]*pub struct' crates/perl-parser/src/ --include='*.rs' 2>/dev/null | wc -l || echo 0)"
    @echo "  pub enum:   $(grep -r '^[[:space:]]*pub enum' crates/perl-parser/src/ --include='*.rs' 2>/dev/null | wc -l || echo 0)"
    @echo ""
    @echo "🔧 LSP Server Size (lsp_server.rs monolith):"
    @echo "  Lines:      $(wc -l < crates/perl-parser/src/lsp_server.rs 2>/dev/null || echo 'N/A')"
    @echo ""
    @echo "💡 Run 'just health-detail' for file-by-file breakdown"

# Detailed health metrics with file breakdown
health-detail:
    @echo "📊 Detailed Health Metrics"
    @echo "=========================="
    @echo ""
    @echo "🔴 Top 10 files with most .unwrap() calls:"
    @grep -r '\.unwrap()' crates/*/src/ --include='*.rs' -c 2>/dev/null | sort -t: -k2 -nr | head -10 || echo "  None found"
    @echo ""
    @echo "🟡 Top 10 files with most eprintln! calls:"
    @grep -r 'eprintln!' crates/*/src/ --include='*.rs' -c 2>/dev/null | sort -t: -k2 -nr | head -10 || echo "  None found"
    @echo ""
    @echo "📁 Largest source files (by lines):"
    @find crates/*/src -name '*.rs' -exec wc -l {} \; 2>/dev/null | sort -nr | head -10 || echo "  None found"

# ============================================================================
# Forensics (post-hoc PR archaeology)
# ============================================================================

# Harvest raw facts from a merged PR
forensics-harvest pr:
    @echo "🔬 Harvesting raw facts from PR {{pr}}..."
    ./scripts/forensics/pr-harvest.sh {{pr}}
    @echo "✅ Harvest complete"

# Compute temporal topology (convergence, friction, oscillations)
forensics-temporal pr:
    @echo "⏱️  Computing temporal topology for PR {{pr}}..."
    ./scripts/forensics/temporal-analysis.sh {{pr}}
    @echo "✅ Temporal analysis complete"

# Run static analysis deltas (quick mode)
forensics-telemetry-quick pr:
    @echo "📊 Running quick telemetry for PR {{pr}}..."
    ./scripts/forensics/telemetry-runner.sh {{pr}} --mode quick
    @echo "✅ Quick telemetry complete"

# Run static analysis deltas (full mode with exhibit-grade tools)
forensics-telemetry-full pr:
    @echo "📊 Running full telemetry for PR {{pr}}..."
    ./scripts/forensics/telemetry-runner.sh {{pr}} --mode full
    @echo "✅ Full telemetry complete"

# Generate complete dossier (runs full pipeline)
forensics-dossier pr:
    @echo "📁 Generating complete dossier for PR {{pr}}..."
    ./scripts/forensics/dossier-runner.sh {{pr}}
    @echo "✅ Dossier generation complete"

# Render dossier markdown from existing YAML outputs
forensics-render pr format='full':
    @echo "📝 Rendering dossier for PR {{pr}} (format: {{format}})..."
    ./scripts/forensics/render-dossier.sh {{pr}} --format {{format}}
    @echo "✅ Rendering complete"
