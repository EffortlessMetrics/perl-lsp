# Justfile for perl-lsp development and CI workflows
# Usage: just <command>
# Install just: cargo install just

# Default recipe (show available commands)
default:
    @just --list

# ============================================================================
# Tiered CI Execution (works locally via Nix and in GitHub Actions)
# ============================================================================
#
# Tier hierarchy:
#   pr-fast    -> Fastest checks for every PR iteration (~1-2 min)
#   merge-gate -> Required before merge to master (~3-5 min)
#   nightly    -> Scheduled comprehensive tests (~15-30 min)
#
# Usage:
#   just pr-fast      # Quick PR validation
#   just merge-gate   # Full pre-merge validation
#   just ci-local     # Same as merge-gate, via Nix
#   nix develop -c just ci-gate  # Canonical local gate

# Helper to time a command and report duration
[private]
_timed name cmd:
    @START=$$(date +%s); \
    echo ">>> Starting {{name}}..."; \
    {{cmd}}; \
    RC=$$?; \
    END=$$(date +%s); \
    DURATION=$$((END - START)); \
    if [ $$RC -eq 0 ]; then \
        echo "<<< {{name}} completed in $${DURATION}s"; \
    else \
        echo "<<< {{name}} FAILED in $${DURATION}s (exit $$RC)"; \
        exit $$RC; \
    fi

# Tier: PR-fast (required for every PR iteration, must be fast ~1-2 min)
pr-fast: _check-tools-basic
    @echo "=============================================="
    @echo "  PR-FAST GATE (quick validation)"
    @echo "=============================================="
    @START=$$(date +%s); \
    just _timed "fmt-check" "just fmt-check" && \
    just _timed "clippy-core" "just clippy-core" && \
    just _timed "test-core" "just test-core"; \
    RC=$$?; \
    END=$$(date +%s); \
    echo ""; \
    echo "=============================================="
    @echo "  PR-fast gate complete (total: $$((END - START))s)"
    @echo "=============================================="
    @exit $$RC

# Tier: Merge-gate (required before merge to master ~3-5 min)
merge-gate: _check-tools-basic pr-fast
    @echo "=============================================="
    @echo "  MERGE GATE (full pre-merge validation)"
    @echo "=============================================="
    @START=$$(date +%s); \
    just _timed "clippy-full" "just clippy-full" && \
    just _timed "test-full" "just test-full" && \
    just _timed "lsp-smoke" "just lsp-smoke" && \
    just _timed "security-audit" "just security-audit" && \
    just _timed "ci-policy" "just ci-policy" && \
    just _timed "ci-v2-bundle-sync" "just ci-v2-bundle-sync" && \
    just _timed "ci-v2-parity" "just ci-v2-parity" && \
    just _timed "ci-lsp-def" "just ci-lsp-def" && \
    just _timed "ci-parser-features-check" "just ci-parser-features-check" && \
    just _timed "ci-features-invariants" "just ci-features-invariants"; \
    RC=$$?; \
    END=$$(date +%s); \
    echo ""; \
    echo "=============================================="
    @if [ $$RC -eq 0 ]; then \
        echo "  Merge gate PASSED (total: $$((END - START))s)"; \
    else \
        echo "  Merge gate FAILED (total: $$((END - START))s)"; \
    fi
    @echo "=============================================="
    @exit $$RC

# Tier: Nightly (scheduled, non-blocking comprehensive tests)
nightly: merge-gate
    @echo "=============================================="
    @echo "  NIGHTLY GATE (comprehensive validation)"
    @echo "=============================================="
    @START=$$(date +%s); \
    just _timed "mutation-subset" "just mutation-subset" && \
    just _timed "fuzz-bounded" "just fuzz-bounded" && \
    just _timed "benchmarks" "just benchmarks"; \
    RC=$$?; \
    END=$$(date +%s); \
    echo ""; \
    echo "=============================================="
    @if [ $$RC -eq 0 ]; then \
        echo "  Nightly gate PASSED (total: $$((END - START))s)"; \
    else \
        echo "  Nightly gate FAILED (total: $$((END - START))s)"; \
    fi
    @echo "=============================================="
    @exit $$RC

# ============================================================================
# Individual Gate Targets
# ============================================================================

# Format check (fast fail)
fmt-check:
    @echo "Checking code formatting..."
    cargo fmt --all -- --check
    @echo "Format check passed"

# Clippy core crates only (fast, for PR iterations)
clippy-core:
    @echo "Running clippy (core crates: perl-parser, perl-lexer)..."
    cargo clippy -p perl-parser -p perl-lexer --locked -- -D warnings -A missing_docs
    @echo "Clippy (core) passed"

# Clippy full workspace (thorough, for merge gate)
clippy-full:
    @echo "Running clippy (full workspace)..."
    cargo clippy --workspace --locked -- -D warnings -A missing_docs
    cargo clippy --workspace --bins --locked --no-deps -- -D clippy::unwrap_used -D clippy::expect_used
    @echo "Clippy (full) passed"

# Test core crates only (fast, for PR iterations)
test-core:
    @echo "Running tests (core crates: perl-parser, perl-lexer)..."
    cargo test -p perl-parser -p perl-lexer --lib --locked
    @echo "Tests (core) passed"

# Test full workspace (thorough, for merge gate)
test-full:
    @echo "Running tests (full workspace)..."
    RUST_TEST_THREADS=2 cargo test --workspace --lib --locked
    @echo "Tests (full) passed"

# LSP smoke test (deterministic, single-threaded)
lsp-smoke:
    @echo "Running LSP smoke tests..."
    cargo test -p perl-lsp --test cli_smoke --locked -- --test-threads=1
    @echo "LSP smoke tests passed"

# Security audit (non-blocking, warns on issues)
security-audit:
    @echo "Running security audit..."
    @if command -v cargo-audit >/dev/null 2>&1; then \
        cargo audit 2>&1 || echo "Audit warnings (non-blocking)"; \
    else \
        echo "SKIP: cargo-audit not installed (run: cargo install cargo-audit)"; \
    fi

# Production hardening security scan
security-hardening:
    @echo "Running production hardening security scan..."
    @./scripts/security-hardening.sh

# Production hardening performance scan
performance-hardening:
    @echo "Running production hardening performance scan..."
    @./scripts/performance-hardening.sh

# Production hardening E2E validation
e2e-validation:
    @echo "Running production hardening E2E validation..."
    @./scripts/e2e-validation.sh

# Complete production hardening validation
production-hardening: security-hardening performance-hardening e2e-validation
    @echo "✅ Production hardening validation completed"
    @echo "📊 Check generated reports for detailed results"

# Production gates validation
production-gates-validation:
    @echo "Running production gates validation..."
    @./scripts/production-gates-validation.sh

# Complete Phase 6 production readiness validation
phase6-production-readiness: production-hardening production-gates-validation
    @echo "🎉 Phase 6 Production Hardening completed!"
    @echo "📋 All security, performance, and validation checks complete"
    @echo "🚀 Ready for v1.0 release validation"

# Generate SBOM in SPDX format
sbom-spdx:
    #!/usr/bin/env bash
    set -euo pipefail
    echo "Generating SBOM (SPDX format)..."
    cargo sbom --output-format spdx_json_2_3 > sbom-spdx.json
    echo "✓ Generated sbom-spdx.json"
    ls -lh sbom-spdx.json

# Generate SBOM in CycloneDX format
sbom-cyclonedx:
    #!/usr/bin/env bash
    set -euo pipefail
    echo "Generating SBOM (CycloneDX format)..."
    cargo sbom --output-format cyclone_dx_json_1_6 > sbom-cyclonedx.json
    echo "✓ Generated sbom-cyclonedx.json"
    ls -lh sbom-cyclonedx.json

# Generate both SBOM formats
sbom: sbom-spdx sbom-cyclonedx
    @echo "✓ Generated both SBOM formats"

# Verify SBOM files
sbom-verify: sbom
    #!/usr/bin/env bash
    set -euo pipefail
    echo "Verifying SBOM files..."
    test -f sbom-spdx.json || (echo "ERROR: sbom-spdx.json not found" && exit 1)
    test -f sbom-cyclonedx.json || (echo "ERROR: sbom-cyclonedx.json not found" && exit 1)
    echo "✓ SBOM files verified"
    ls -lh sbom-*.json

# ============================================================================
# Heavy Jobs (label-gated in CI, for nightly tier)
# ============================================================================

# Mutation testing subset (bounded, ~5-10 min)
mutation-subset:
    @echo "Running mutation testing (subset)..."
    @if command -v cargo-mutants >/dev/null 2>&1; then \
        cargo mutants --workspace -j 2 --timeout 60 2>&1 || echo "Mutation testing completed (some mutants may survive)"; \
    else \
        echo "SKIP: cargo-mutants not installed (run: cargo install cargo-mutants)"; \
    fi

# Bounded fuzz run (quick fuzzing for CI/nightly)
fuzz-bounded:
    @echo "🔥 Running bounded fuzz testing (60 seconds per target)..."
    @cargo +nightly fuzz run builtin_functions -- -max_total_time=60 || echo "  Builtin functions fuzzing complete"
    @cargo +nightly fuzz run heredoc_parsing -- -max_total_time=60 || echo "  Heredoc fuzzing complete"
    @cargo +nightly fuzz run substitution_parsing -- -max_total_time=60 || echo "  Substitution fuzzing complete"
    @echo "✅ Fuzz testing complete"

# Benchmarks (requires criterion) - legacy target, prefer 'just bench'
benchmarks:
    @echo "Running benchmarks..."
    @mkdir -p benchmarks/results
    @if cargo bench --workspace --locked --no-run 2>/dev/null; then \
        cargo bench --workspace --locked -- --noplot 2>&1 | tee benchmarks/results/raw-output.txt || echo "Benchmark run completed"; \
        echo ""; \
        echo "For structured results, run: just bench"; \
    else \
        echo "SKIP: No benchmarks configured or build failed"; \
    fi

# ============================================================================
# CI Aliases and Convenience Targets
# ============================================================================

# Canonical local gate via Nix (recommended for pre-push)
ci-local:
    @echo "Running ci-gate via Nix shell..."
    @if command -v nix >/dev/null 2>&1; then \
        nix develop -c just ci-gate; \
    else \
        echo "ERROR: Nix not found. Install Nix or run 'just ci-gate' directly."; \
        echo "  Install Nix: https://nixos.org/download.html"; \
        exit 1; \
    fi

# Tool availability check (basic tools for PR-fast)
[private]
_check-tools-basic:
    @MISSING=""; \
    if ! command -v cargo >/dev/null 2>&1; then MISSING="$$MISSING cargo"; fi; \
    if ! command -v rustfmt >/dev/null 2>&1; then MISSING="$$MISSING rustfmt"; fi; \
    if [ -n "$$MISSING" ]; then \
        echo "ERROR: Missing required tools:$$MISSING"; \
        echo "  Install Rust: https://rustup.rs"; \
        exit 1; \
    fi

# ============================================================================
# CI Validation Commands (Issue #211)
# ============================================================================

# MSRV: Rust 1.92 (for OpenAI Codex compatibility)
# The rust-toolchain.toml pins to 1.92.0, so standard commands use MSRV by default.
# Use these recipes to explicitly verify MSRV compliance:

# Phase 0: publish receipts to review/receipts/YYYY-MM-DD/
receipts date='':
    @d="{{date}}"; \
    if [ -z "$$d" ]; then d="$$(date -u +%Y-%m-%d)"; fi; \
    echo "Publishing receipts for $$d"; \
    bash scripts/publish-receipts.sh "$$d"

# Issue #211: measure CI lane runtimes locally (baseline before cleanup)
ci-measure:
    @echo "Measuring CI lane runtimes..."
    @bash .ci/scripts/measure-ci-time.sh

# Fast merge gate on MSRV (~2-5 min) - proves 1.92 compatibility
ci-gate-msrv:
    @echo "🚪 Running fast merge gate on MSRV (Rust 1.92)..."
    @RUSTUP_TOOLCHAIN=1.92.0 just ci-gate

# Low-memory merge gate - for constrained environments (WSL, CI runners, low-RAM)
# Forces single-threaded builds/tests to prevent OOM crashes
# Key fixes: unset RUSTC_WRAPPER (not empty), --no-deps on clippy
ci-gate-low-mem:
    @echo "🚪 Running low-memory merge gate (sequential, single-threaded)..."
    @echo "   Using CARGO_BUILD_JOBS=1, RUST_TEST_THREADS=1, RUSTC_WRAPPER unset"
    @env -u RUSTC_WRAPPER CARGO_BUILD_JOBS=1 RUST_TEST_THREADS=1 PROPTEST_CASES=32 \
        sh -c 'just ci-workflow-audit && \
        just ci-check-no-nested-lock && \
        just ci-format && \
        just ci-docs-check && \
        echo "🔍 Running clippy (single-threaded, no-deps)..." && \
        cargo clippy --workspace --lib --locked --no-deps -j1 -- -D warnings -A missing_docs && \
        cargo clippy --workspace --bins --locked --no-deps -j1 -- -D clippy::unwrap_used -D clippy::expect_used && \
        just ci-forbid-fatal && \
        echo "🧪 Running library tests (single-threaded)..." && \
        cargo test --workspace --lib --locked -j1 -- --test-threads=1 && \
        just ci-policy && \
        just ci-lsp-def && \
        just ci-parser-features-check && \
        just ci-features-invariants'
    @echo "✅ Low-memory merge gate passed!"

# Full CI on MSRV (~10-20 min) - proves 1.92 compatibility for releases
ci-full-msrv:
    @echo "🚀 Running full CI on MSRV (Rust 1.92)..."
    @RUSTUP_TOOLCHAIN=1.92.0 just ci-full

# Check for nested Cargo.lock files (footgun prevention)
ci-check-no-nested-lock:
    @echo "🔒 Checking for nested Cargo.lock files..."
    @if find . -name 'Cargo.lock' -type f \
        -not -path '*/target/*' \
        -not -path '*/.runs/*' \
        -not -path '*/archive/*' \
        2>/dev/null | grep -v '^\./Cargo\.lock$' | grep -q .; then \
        echo "❌ ERROR: Nested Cargo.lock detected! Run gates from repo root only."; \
        find . -name 'Cargo.lock' -type f \
            -not -path '*/target/*' \
            -not -path '*/.runs/*' \
            -not -path '*/archive/*' \
            2>/dev/null | grep -v '^\./Cargo\.lock$'; \
        exit 1; \
    fi
    @echo "✅ No nested lockfiles"

# Audit workflows for ungated expensive jobs
ci-workflow-audit:
    @python3 scripts/ci-audit-workflows.py

# Fast merge gate (~2-5 min) - REQUIRED for all merges
# This is the canonical pre-push check (same as merge-gate with legacy checks)
ci-gate:
    @echo "Running fast merge gate..."
    just ci-workflow-audit && \
    just ci-check-no-nested-lock && \
    just ci-format && \
    just ci-docs-check && \
    just ci-clippy-lib && \
    just clippy-prod-no-unwrap && \
    just clippy-no-unwrap-all && \
    just ci-unwrap-panic-ratchet && \
    just ci-unsafe-ratchet && \
    just ci-forbid-fatal && \
    just ci-test-lib && \
    just ci-policy && \
    just ci-v2-bundle-sync && \
    just ci-v2-parity && \
    just ci-lsp-def && \
    just ci-lsp-smoke-e2e && \
    just ci-semantic-frameworks && \
    just ci-dap-smoke-e2e && \
    just ci-parser-features-check && \
    just ci-features-invariants
    # @START=$$(date +%s); \

# Gate runner with receipt output (Issue #210)
# Uses xtask gates for structured gate execution with receipt generation
gates tier='merge-gate' *args='':
    @echo "🧾 Running gate runner (tier: {{tier}})..."
    cargo xtask gates --tier {{tier}} --receipt {{args}}

# Run gates with JSON output (for CI)
gates-json tier='merge-gate':
    @cargo xtask gates --tier {{tier}} --format json --receipt

# List available gates
gates-list:
    @cargo xtask gates --list

# Run old shell-based gate runner (deprecated, kept for compatibility)
gates-legacy:
    @echo "🧾 Running legacy gate runner..."
    @bash scripts/run-gates.sh

# Full CI pipeline (~10-20 min) - RECOMMENDED for large changes
ci-full:
    @echo "🚀 Running full CI pipeline..."
    @just ci-format
    @just ci-docs-check
    @just ci-clippy
    @just ci-test-core
    @just ci-test-lsp
    @just ci-lsp-bdd
    @just ci-docs
    @echo "✅ Full CI passed!"

# Local CI parity with .github/workflows/ci.yml (legacy alias)
# Prefer: nix develop -c just ci-gate
ci-local-full:
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

# Clippy production unwrap/expect gate (Issue #143) - prevents panic-prone code in shipped binaries
clippy-prod-no-unwrap:
    @echo "🔒 Enforcing no unwrap/expect in production code..."
    cargo clippy --workspace --lib --bins --no-deps -- -D clippy::unwrap_used -D clippy::expect_used

# Clippy NO UNWRAP ALL gate - enforces zero unwrap/expect everywhere
clippy-no-unwrap-all:
    @echo "🔒 Enforcing no unwrap/expect everywhere (including tests)..."
    cargo clippy --workspace --all-targets -- -D clippy::unwrap_used -D clippy::expect_used
    @echo "✅ Production code is panic-safe (no unwrap/expect)"

# Unwrap/panic-family ratchet (production source only)
ci-unwrap-panic-ratchet:
    @echo "🛡️  Checking unwrap/panic-family ratchet..."
    @bash ci/check_unwraps_prod.sh
    @echo "✅ Unwrap/panic-family ratchet passed"

# Unsafe syntax ratchet (production source only)
ci-unsafe-ratchet:
    @echo "🛡️  Checking unsafe syntax ratchet..."
    @bash ci/check_unsafe_prod.sh
    @echo "✅ Unsafe syntax ratchet passed"

# Forbid fatal constructs gate - catches abort/exit/panic that Clippy misses
ci-forbid-fatal:
    @echo "🚫 Checking for forbidden fatal constructs..."
    @bash scripts/forbid-fatal-constructs.sh --verbose
    @echo "✅ No forbidden fatal constructs"

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

# V2 bundle sync guard (in-crate v2 files must match extracted perl-parser-pest v2 files)
ci-v2-bundle-sync:
    @echo "🔍 Checking v2 bundle sync..."
    bash scripts/check-v2-bundle-sync.sh
    @echo "✅ V2 bundle sync check passed"

# V2 parser parity guard (in-crate v2 vs extracted perl-parser-pest v2)
ci-v2-parity:
    @echo "🧪 Running v2 parity corpus check..."
    cargo run --locked -p xtask --features legacy -- corpus --scanner v2-parity
    @echo "✅ V2 parity corpus check passed"

# Targeted parser/DAP verification (low-memory, for heredoc/breakpoint changes)
# Key fixes: unset RUSTC_WRAPPER (not empty), --no-deps on clippy, targeted tests
ci-test-parser-dap:
    @echo "🎯 Running targeted parser/DAP tests (single-threaded)..."
    @env -u RUSTC_WRAPPER CARGO_BUILD_JOBS=1 RUST_TEST_THREADS=1 \
        sh -c 'echo "📦 Building perl-parser-core..." && \
        cargo build -p perl-parser-core --lib -j1 && \
        echo "🧪 Running perl-parser heredoc tests..." && \
        cargo test -p perl-parser -j1 -- --test-threads=1 heredoc && \
        echo "🧪 Running DAP breakpoint tests..." && \
        cargo test -p perl-dap --test dap_breakpoint_matrix_tests -j1 -- --test-threads=1 && \
        echo "🔍 Running clippy on affected crates (no-deps)..." && \
        cargo clippy -p perl-parser-core -p perl-parser -p perl-dap --lib --no-deps -j1 -- -D warnings'
    @echo "✅ Parser/DAP tests passed"

# LSP integration tests (with adaptive threading)
ci-test-lsp:
    @echo "🔌 Running LSP integration tests..."
    RUST_TEST_THREADS=2 cargo test -p perl-lsp --test lsp_comprehensive_e2e_test -- --test-threads=2
    @echo "✅ LSP tests passed"

# LSP semantic definition tests (semantic-aware go-to-definition)
ci-lsp-def:
    @echo "🔎 Running LSP semantic definition tests..."
    @env -u RUSTC_WRAPPER RUST_TEST_THREADS=1 CARGO_BUILD_JOBS=1 \
        cargo test -p perl-lsp --test semantic_definition -- --test-threads=1
    @echo "✅ LSP semantic definition tests passed"

# LSP process-level smoke receipt (initialize/open/completion/hover/definition/shutdown)
ci-lsp-smoke-e2e:
    @echo "💨 Running LSP stdio smoke E2E test..."
    @env -u RUSTC_WRAPPER RUST_TEST_THREADS=1 CARGO_BUILD_JOBS=1 \
        cargo test -p perl-lsp --test lsp_smoke_e2e -- --test-threads=1
    @echo "✅ LSP smoke E2E passed"

# LSP BDD workflow tests (serialized to prevent WSL resource exhaustion)
ci-lsp-bdd:
    @echo "🎭 Running LSP BDD workflow tests..."
    @env -u RUSTC_WRAPPER RUST_TEST_THREADS=1 CARGO_BUILD_JOBS=1 \
        cargo test -p perl-lsp --test lsp_bdd_workflows -- --test-threads=1
    @echo "✅ LSP BDD workflow tests passed"

# Framework semantic depth receipts (Moo/Moose/Class::Accessor)
ci-semantic-frameworks:
    @echo "🧠 Running framework semantic tests..."
    @env -u RUSTC_WRAPPER RUST_TEST_THREADS=1 CARGO_BUILD_JOBS=1 \
        cargo test -p perl-semantic-analyzer --test frameworks_moo -- --test-threads=1
    @env -u RUSTC_WRAPPER RUST_TEST_THREADS=1 CARGO_BUILD_JOBS=1 \
        cargo test -p perl-lsp --test moo_semantics_e2e -- --test-threads=1
    @echo "✅ Framework semantic tests passed"

# DAP smoke receipt (launch/breakpoint/step/stack/evaluate/disconnect)
ci-dap-smoke-e2e:
    @echo "🐞 Running DAP smoke E2E test..."
    @env -u RUSTC_WRAPPER RUST_TEST_THREADS=1 CARGO_BUILD_JOBS=1 \
        cargo test -p perl-dap --test dap_smoke_e2e -- --test-threads=1
    @echo "✅ DAP smoke E2E passed"

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
# Low-Memory Debugging Commands
# ============================================================================

# Trace a command with /usr/bin/time -v to capture Max RSS (peak memory)
# Usage: just trace 'cargo clippy -p perl-parser --no-deps -j1 -- -D warnings'
trace cmd:
    @mkdir -p target/ci-trace
    @bash -c 'set -euo pipefail; \
      log=target/ci-trace/trace-$(date +%Y%m%d-%H%M%S).log; \
      echo "CMD: {{cmd}}" | tee -a "$$log"; \
      /usr/bin/time -v {{cmd}} 2>&1 | tee -a "$$log"; \
      echo "---" | tee -a "$$log"; \
      echo "Log: $$log"'

# Trace each low-mem step individually to find memory hotspots
trace-lowmem-steps:
    @echo "🔬 Tracing low-memory steps individually..."
    @mkdir -p target/ci-trace
    @echo "Step 1: format check"
    @just trace 'cargo fmt --check --all'
    @echo "Step 2: clippy lib (no-deps)"
    @just trace 'env -u RUSTC_WRAPPER cargo clippy --workspace --lib --locked --no-deps -j1 -- -D warnings -A missing_docs'
    @echo "Step 3: clippy bins (no-deps)"
    @just trace 'env -u RUSTC_WRAPPER cargo clippy --workspace --bins --locked --no-deps -j1 -- -D clippy::unwrap_used -D clippy::expect_used'
    @echo "Step 4: tests lib"
    @just trace 'env -u RUSTC_WRAPPER RUST_TEST_THREADS=1 cargo test --workspace --lib --locked -j1 -- --test-threads=1'
    @echo "📊 Check target/ci-trace/ for Max RSS values"

# Full parser/DAP tests (not just heredoc-targeted) with low-memory settings
ci-test-parser-dap-full:
    @echo "🎯 Running full parser/DAP tests (single-threaded)..."
    @env -u RUSTC_WRAPPER CARGO_BUILD_JOBS=1 RUST_TEST_THREADS=1 \
        sh -c 'echo "🧪 Running all perl-parser lib tests..." && \
        cargo test -p perl-parser --lib -j1 -- --test-threads=1 && \
        echo "🧪 Running all perl-dap tests..." && \
        cargo test -p perl-dap -j1 -- --test-threads=1 && \
        echo "🔍 Running clippy on affected crates (no-deps)..." && \
        cargo clippy -p perl-parser-core -p perl-parser -p perl-dap --lib --no-deps -j1 -- -D warnings'
    @echo "✅ Full Parser/DAP tests passed"

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

# Policy and governance checks
ci-policy:
    @echo "⚖️  Checking project policies..."
    just ci-check-todos
    @bash ./.ci/scripts/check-from-raw.sh
    @python3 scripts/update-current-status.py --check

# Check for machine-specific paths in documentation
ci-doc-paths:
    @echo "🔍 Checking documentation paths..."
    @bash ci/check_doc_paths.sh docs
    @echo "✅ Documentation paths check passed"

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
# Parser Feature Coverage Commands (Issue #180)
# ============================================================================

# Run parser audit for coverage analysis (detailed report)
parser-audit:
    @echo "📊 Running parser audit..."
    @cargo run -p xtask --no-default-features -- corpus-audit --fresh --corpus-path .
    @echo ""
    @echo "Report written to: corpus_audit_report.json"
    @python3 -c "import json; r=json.load(open('corpus_audit_report.json')); po=r['parse_outcomes']; print(f'Parse success: {po[\"ok\"]}/{po[\"total\"]} files ({100*po[\"ok\"]/po[\"total\"]:.0f}%)')"

# Check parser features baseline (CI mode, fails on regression)
ci-parser-features-check:
    @echo "🔍 Checking parser features baseline..."
    @bash ci/check_parse_errors.sh

# Check features.toml invariants (GA+advertised must have tests, no duplicates)
ci-features-invariants:
    @echo "🔍 Checking features.toml invariants..."
    @python3 scripts/check_features_invariants.py

# Update parser feature matrix document from audit report
parser-matrix-update:
    @echo "📝 Updating parser feature matrix..."
    @python3 scripts/update-parser-matrix.py

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
# Roadmap Gate (informational, never blocks merge)
# ============================================================================

# Run feature/infra ignored tests and report progress
roadmap-gate:
    @echo "=== ROADMAP BACKLOG: running ignored feature/infra tests ==="
    -cargo test -p perl-semantic-analyzer -- test_anonymous_subroutine --ignored --nocapture
    -cargo test -p perl-dap -- test_attach_tcp_valid_arguments test_attach_default_values --ignored --nocapture
    -cargo test -p perl-parser -- test_statement_with_or_modifier --ignored --nocapture
    -RUST_TEST_THREADS=2 cargo test -p perl-lsp -- test_fix_undefined_variable test_user_story_debugging_workflow test_user_story_refactoring_legacy_code --ignored --test-threads=2 --nocapture
    @echo "=== Roadmap gate complete (failures = unimplemented features) ==="

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
    @echo "🔧 LSP Crate Size (crates/perl-lsp/src/):"
    @echo "  Lines:      $(find crates/perl-lsp/src -name '*.rs' | xargs wc -l | tail -n 1 | awk '{print $1}' || echo 'N/A')"
    @echo ""
    @echo "🧹 Dead Code Metrics:"
    @echo "  Unused deps: $(cargo machete 2>&1 | grep -c 'Cargo.toml:' || echo 0) crates affected"
    @echo "  Dead code allows: $(grep -r '#\[allow(dead_code)\]' crates --include='*.rs' 2>/dev/null | wc -l || echo 0)"
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
# Milestone Verification
# ============================================================================

# Verify v0.9.0 release exit criteria (mechanical checks)
milestone-v0_9-check:
    @echo "🎯 Verifying v0.9.0 exit criteria..."
    @echo ""
    @echo "📋 Step 1: Running ci-gate..."
    @just ci-gate
    @echo ""
    @echo "📋 Step 2: Checking ignored test breakdown..."
    @bash scripts/ignored-test-count.sh
    @echo ""
    @echo "📋 Step 3: Verifying metrics consistency..."
    @just status-check
    @echo ""
    @echo "✅ v0.9.0 exit criteria check complete!"
    @echo "   Next: Manual review of BUG=0, MANUAL≤1 from test count output above"

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

# ============================================================================
# Benchmark Infrastructure
# ============================================================================
# Run performance benchmarks with structured output.
# See benchmarks/README.md for documentation.

# Run all benchmarks
bench:
    @echo "📊 Running full benchmark suite..."
    @mkdir -p benchmarks/results
    ./benchmarks/scripts/run-benchmarks.sh --output benchmarks/results/latest.json
    @echo ""
    @echo "Results saved to benchmarks/results/latest.json"
    @python3 ./benchmarks/scripts/format-results.py benchmarks/results/latest.json

# Quick smoke benchmarks (fast, ~30s)
bench-quick:
    @echo "⚡ Running quick benchmark smoke test..."
    @mkdir -p benchmarks/results
    ./benchmarks/scripts/run-benchmarks.sh --quick --output benchmarks/results/latest.json
    @echo ""
    @python3 ./benchmarks/scripts/format-results.py benchmarks/results/latest.json --receipt

# Compare current results against baseline
bench-compare:
    @echo "📈 Comparing against baseline..."
    ./benchmarks/scripts/compare.sh

# Compare with failure on regression (for CI)
bench-compare-strict:
    @echo "📈 Comparing against baseline (strict mode)..."
    ./benchmarks/scripts/compare.sh --fail-on-regression

# Save current results as a new baseline
bench-baseline version='':
    @echo "📝 Saving benchmark baseline..."
    @mkdir -p benchmarks/baselines
    @if [ -z "{{version}}" ]; then \
        VERSION="v$(date +%Y%m%d)"; \
    else \
        VERSION="{{version}}"; \
    fi; \
    if [ ! -f benchmarks/results/latest.json ]; then \
        echo "No results found. Running benchmarks first..."; \
        just bench; \
    fi; \
    cp benchmarks/results/latest.json "benchmarks/baselines/$$VERSION.json"; \
    echo "Baseline saved to benchmarks/baselines/$$VERSION.json"

# Run parser benchmarks only
bench-parser:
    @echo "📊 Running parser benchmarks..."
    ./benchmarks/scripts/run-benchmarks.sh --category parser

# Run lexer benchmarks only
bench-lexer:
    @echo "📊 Running lexer benchmarks..."
    ./benchmarks/scripts/run-benchmarks.sh --category lexer

# Run LSP benchmarks only
bench-lsp:
    @echo "📊 Running LSP benchmarks..."
    ./benchmarks/scripts/run-benchmarks.sh --category lsp

# Run workspace index benchmarks only
bench-index:
    @echo "📊 Running workspace index benchmarks..."
    ./benchmarks/scripts/run-benchmarks.sh --category index

# Format benchmark results as receipt
bench-receipt:
    @echo "📋 Generating benchmark receipt..."
    @python3 ./benchmarks/scripts/format-results.py benchmarks/results/latest.json --receipt

# Format benchmark results as markdown
bench-markdown:
    @echo "📋 Generating benchmark markdown..."
    @python3 ./benchmarks/scripts/format-results.py benchmarks/results/latest.json --markdown

# Generate performance regression alerts (terminal)
bench-alert:
    @echo "📊 Checking for performance regressions..."
    @python3 ./benchmarks/scripts/alert.py

# Generate performance regression alerts (markdown for PR)
bench-alert-md:
    @echo "📊 Generating performance alert (markdown)..."
    @python3 ./benchmarks/scripts/alert.py --format markdown

# Check for critical performance regressions (exits non-zero)
bench-alert-check:
    @echo "🔍 Checking for critical regressions..."
    @python3 ./benchmarks/scripts/alert.py --check

# ============================================================================
# Code Coverage (Issue #276)
# ============================================================================
# Generate and analyze code coverage reports using cargo-llvm-cov.
# See codecov.yml for service configuration.

# Generate local HTML coverage report
coverage:
    @echo "📊 Generating coverage report..."
    @if ! command -v cargo-llvm-cov >/dev/null 2>&1; then \
        echo "❌ cargo-llvm-cov not found. Installing..."; \
        cargo install cargo-llvm-cov --locked; \
    fi
    @cargo llvm-cov --workspace --locked --exclude xtask --html --output-dir target/coverage \
        --ignore-filename-regex '(archive|tree-sitter-perl-rs|tree-sitter-perl-c|tests|benches|examples|build\.rs)/'
    @echo "✅ Coverage report: target/coverage/index.html"
    @echo "📈 Opening report in browser..."
    @command -v xdg-open >/dev/null 2>&1 && xdg-open target/coverage/index.html || \
     command -v open >/dev/null 2>&1 && open target/coverage/index.html || \
     echo "⚠️  Please open target/coverage/index.html manually"

# Generate coverage report (lcov format for CI)
coverage-lcov:
    @echo "📊 Generating coverage (lcov format)..."
    @if ! command -v cargo-llvm-cov >/dev/null 2>&1; then \
        echo "❌ cargo-llvm-cov not found. Installing..."; \
        cargo install cargo-llvm-cov --locked; \
    fi
    @cargo llvm-cov --workspace --locked --exclude xtask --lcov --output-path lcov.info \
        --ignore-filename-regex '(archive|tree-sitter-perl-rs|tree-sitter-perl-c|tests|benches|examples|build\.rs)/'
    @echo "✅ Coverage: lcov.info"

# Show coverage summary (terminal)
coverage-summary:
    @echo "📊 Coverage Summary"
    @echo "==================="
    @if ! command -v cargo-llvm-cov >/dev/null 2>&1; then \
        echo "❌ cargo-llvm-cov not found. Installing..."; \
        cargo install cargo-llvm-cov --locked; \
    fi
    @cargo llvm-cov --workspace --locked --exclude xtask \
        --ignore-filename-regex '(archive|tree-sitter-perl-rs|tree-sitter-perl-c|tests|benches|examples|build\.rs)/'

# ============================================================================
# Technical Debt Tracking (Issue #XXX)
# ============================================================================
# Track flaky tests, known issues, and technical debt with budgets.
# See .ci/debt-ledger.yaml for configuration.

# Show current debt status report
debt-report:
    @echo "📊 Technical Debt Report"
    @python3 scripts/debt-report.py

# CI gate: fail if debt budget exceeded or quarantines expired
debt-check:
    @echo "🔍 Checking debt budget compliance..."
    @python3 scripts/debt-report.py --check

# Show only expired quarantines (quick check)
debt-expired:
    @python3 scripts/debt-report.py --expired

# Output debt report as JSON (for receipt integration)
debt-json:
    @python3 scripts/debt-report.py --json

# Add a flaky test to quarantine (interactive helper)
debt-quarantine name issue days="14":
    @echo "Adding {{name}} to quarantine for {{days}} days..."
    @echo ""
    @echo "To complete this action, add the following to .ci/debt-ledger.yaml"
    @echo "under the 'flaky_tests:' section:"
    @echo ""
    @echo "  - name: \"{{name}}\""
    @echo "    added: \"$(date -u +%Y-%m-%d)\""
    @echo "    issue: \"{{issue}}\""
    @echo "    tier: \"quarantine\""
    @echo "    quarantine_days: {{days}}"
    @echo "    expires: \"$(date -u -d '+{{days}} days' +%Y-%m-%d 2>/dev/null || date -v+{{days}}d -u +%Y-%m-%d)\""
    @echo "    notes: \"<describe the failure pattern>\""
    @echo ""
    @echo "Then run: just debt-report"

# Remove a test from quarantine (interactive helper)
debt-unquarantine name:
    @echo "To remove {{name}} from quarantine:"
    @echo ""
    @echo "1. Remove the entry from .ci/debt-ledger.yaml 'flaky_tests:' section"
    @echo "2. Optionally add a 'resolved' entry to the 'history.resolved:' section:"
    @echo ""
    @echo "  - type: \"flaky_test\""
    @echo "    name: \"{{name}}\""
    @echo "    resolved: \"$(date -u +%Y-%m-%d)\""
    @echo "    resolution: \"<describe the fix>\""
    @echo "    pr: \"#XXX\""
    @echo ""
    @echo "3. Run: just debt-report"

# Show debt summary suitable for PR comments
debt-pr-summary:
    @echo "## Technical Debt Status"
    @echo ""
    @python3 scripts/debt-report.py --json | python3 scripts/debt-pr-summary.py

# ============================================================================
# CI Guardrail Tests (Issue #364)
# ============================================================================
# Tests for automated ignored test monitoring and governance.
# Tests are in xtask/tests/ci_guardrail_ignored_test_monitoring_tests.rs

# Run guardrail tests (shows ignored status)
guardrail-tests:
    @echo "🔍 Running CI guardrail tests (scaffolding)..."
    cargo test -p xtask --test ci_guardrail_ignored_test_monitoring_tests

# Check guardrail test status
guardrail-status:
    @echo "📊 CI Guardrail Test Status"
    @echo "==========================="
    @echo ""
    @cargo test -p xtask --test ci_guardrail_ignored_test_monitoring_tests 2>&1 | grep -E "(test .*ignored|test result)"
    @echo ""
    @echo "Note: These tests are scaffolding for Issue #364"
    @echo "They will be enabled as features are implemented (AC13-AC15)"

# Try running guardrail tests (will fail until features implemented)
guardrail-run-ignored:
    @echo "⚠️  Attempting to run ignored guardrail tests..."
    @echo "Note: Some tests expected to fail pending feature implementation"
    @cargo test -p xtask --test ci_guardrail_ignored_test_monitoring_tests -- --ignored || true

# ============================================================================
# SemVer Breaking Change Detection (Issue #277)
# ============================================================================
# Automated semantic versioning validation to prevent accidental breaking changes.
# Uses cargo-semver-checks to compare against baseline (last release tag).

# Check for breaking changes against last release
semver-check:
    @echo "🔍 Checking for SemVer breaking changes..."
    @just _semver-check-install
    @just _semver-check-run

# Check specific package for breaking changes
semver-check-package package:
    @echo "🔍 Checking {{package}} for SemVer breaking changes..."
    @just _semver-check-install
    @cargo semver-checks check-release -p {{package}} --baseline-rev $(just _semver-baseline-tag)

# Check all published packages
semver-check-all:
    @echo "🔍 Checking all published packages for SemVer breaking changes..."
    @just _semver-check-install
    @just semver-check-package perl-parser
    @just semver-check-package perl-lexer
    @just semver-check-package perl-parser-core
    @just semver-check-package perl-lsp

# Generate breaking changes report
semver-report:
    @echo "📊 Generating SemVer breaking changes report..."
    @just _semver-check-install
    @mkdir -p target/semver-reports
    @cargo semver-checks check-release --workspace --baseline-rev $(just _semver-baseline-tag) \
        --output-format json > target/semver-reports/breaking-changes.json || true
    @echo "Report saved to: target/semver-reports/breaking-changes.json"

# List all available baseline tags
semver-list-baselines:
    @echo "📋 Available baseline tags:"
    @git tag | grep -E '^v[0-9]+\.[0-9]+\.[0-9]+$' | sort -V | tail -10

# Show what changed in public API since last release
semver-diff package='perl-parser':
    @echo "📝 Public API changes in {{package}} since last release:"
    @just _semver-check-install
    @cargo semver-checks check-release -p {{package}} --baseline-rev $(just _semver-baseline-tag) || true

# Private helper: install cargo-semver-checks if missing
[private]
_semver-check-install:
    @if ! command -v cargo-semver-checks >/dev/null 2>&1; then \
        echo "📦 Installing cargo-semver-checks..."; \
        cargo install cargo-semver-checks --locked; \
    fi

# Private helper: run semver checks on core packages
[private]
_semver-check-run:
    @BASELINE=$(just _semver-baseline-tag); \
    echo "Using baseline: $$BASELINE"; \
    echo ""; \
    echo "Checking perl-parser..."; \
    cargo semver-checks check-release -p perl-parser --baseline-rev "$$BASELINE" || EXIT_CODE=1; \
    echo ""; \
    echo "Checking perl-lexer..."; \
    cargo semver-checks check-release -p perl-lexer --baseline-rev "$$BASELINE" || EXIT_CODE=1; \
    echo ""; \
    echo "Checking perl-parser-core..."; \
    cargo semver-checks check-release -p perl-parser-core --baseline-rev "$$BASELINE" || EXIT_CODE=1; \
    exit $${EXIT_CODE:-0}

# Private helper: get baseline tag for comparison
[private]
_semver-baseline-tag:
    @git tag | grep -E '^v[0-9]+\.[0-9]+\.[0-9]+$' | sort -V | tail -1

# ============================================================================
# Fuzzing (cargo-fuzz integration)
# ============================================================================

# Run fuzzing on specific target (default: 60 seconds)
fuzz target='substitution_parsing' duration='60':
    @echo "🔥 Fuzzing {{target}} for {{duration}} seconds..."
    @cargo +nightly fuzz run {{target}} -- -max_total_time={{duration}}

# List available fuzz targets
fuzz-list:
    @echo "📋 Available fuzz targets:"
    @cargo +nightly fuzz list

# Run continuous fuzzing (for local development, Ctrl+C to stop)
fuzz-continuous target='substitution_parsing':
    @echo "🔥 Running continuous fuzzing on {{target}} (Ctrl+C to stop)..."
    @echo "📊 Corpus: fuzz/corpus/{{target}}"
    @echo "💥 Crashes: fuzz/artifacts/{{target}}"
    @cargo +nightly fuzz run {{target}}

# Check fuzz corpus coverage for a target
fuzz-coverage target='substitution_parsing':
    @echo "📊 Checking coverage for {{target}}..."
    @cargo +nightly fuzz coverage {{target}}
    @echo ""
    @echo "To view coverage report, open: fuzz/coverage/{{target}}/coverage/index.html"

# Minimize a crash case to smallest reproducing input
fuzz-minimize target crash:
    @echo "🔍 Minimizing crash case for {{target}}..."
    @cargo +nightly fuzz cmin {{target}} {{crash}}

# Check for crash artifacts (fails if crashes found)
fuzz-check-crashes:
    @echo "💥 Checking for crash artifacts..."
    @if [ -d fuzz/artifacts ]; then \
        CRASHES=$$(find fuzz/artifacts -type f 2>/dev/null | wc -l); \
        if [ $$CRASHES -gt 0 ]; then \
            echo "⚠️  Found $$CRASHES crash artifacts:"; \
            find fuzz/artifacts -type f 2>/dev/null; \
            exit 1; \
        else \
            echo "✅ No crashes found"; \
        fi; \
    else \
        echo "✅ No artifacts directory (no crashes)"; \
    fi

# Run all fuzz targets for regression testing (short duration)
fuzz-regression duration='30':
    @echo "🔥 Running fuzz regression tests ({{duration}}s per target)..."
    @just fuzz builtin_functions {{duration}} || true
    @just fuzz heredoc_parsing {{duration}} || true
    @just fuzz substitution_parsing {{duration}} || true
    @just fuzz lsp_navigation {{duration}} || true
    @just fuzz unicode_positions {{duration}} || true
    @just fuzz-check-crashes
    @echo "✅ Fuzz regression testing complete"

# ============================================================================
# Documentation Site (mdBook)
# ============================================================================

# Build documentation site with mdBook
docs-build:
    @echo "📖 Building mdBook documentation site..."
    @bash scripts/populate-book.sh
    mdbook build book
    @echo "✅ Documentation site built successfully"
    @echo "📂 Output: book/book/index.html"

# Serve documentation site locally
docs-serve:
    @echo "📖 Serving mdBook documentation site..."
    @bash scripts/populate-book.sh
    @echo "🌐 Starting local server at http://localhost:3000"
    @echo "Press Ctrl+C to stop"
    mdbook serve book --port 3000 --open

# Clean documentation build artifacts
docs-clean:
    @echo "🧹 Cleaning documentation build artifacts..."
    rm -rf book/book
    rm -rf book/src/getting-started
    rm -rf book/src/user-guides
    rm -rf book/src/architecture
    rm -rf book/src/developer
    rm -rf book/src/lsp
    rm -rf book/src/advanced
    rm -rf book/src/reference
    rm -rf book/src/dap
    rm -rf book/src/ci
    rm -rf book/src/process
    rm -rf book/src/resources
    @echo "✅ Documentation artifacts cleaned"

# ============================================================================
# Changelog Generation (Issue #280)
# ============================================================================
# Automated changelog generation using git-cliff.
# See cliff.toml for configuration.

# Generate full changelog (overwrites CHANGELOG.md)
changelog:
    @echo "📝 Generating changelog..."
    @if command -v git-cliff >/dev/null 2>&1; then \
        git-cliff --output CHANGELOG.md; \
        echo "✅ Changelog generated: CHANGELOG.md"; \
    else \
        echo "ERROR: git-cliff not installed."; \
        echo "  Install via: cargo install git-cliff"; \
        echo "  Or: brew install git-cliff (macOS)"; \
        echo "  Or: nix-shell -p git-cliff (Nix)"; \
        exit 1; \
    fi

# Generate changelog for unreleased changes only (preview mode)
changelog-preview:
    @echo "📋 Previewing unreleased changes..."
    @if command -v git-cliff >/dev/null 2>&1; then \
        git-cliff --unreleased; \
    else \
        echo "ERROR: git-cliff not installed. Run: cargo install git-cliff"; \
        exit 1; \
    fi

# Generate changelog for a specific version range
changelog-range from to:
    @echo "📋 Generating changelog from {{from}} to {{to}}..."
    @if command -v git-cliff >/dev/null 2>&1; then \
        git-cliff {{from}}..{{to}}; \
    else \
        echo "ERROR: git-cliff not installed. Run: cargo install git-cliff"; \
        exit 1; \
    fi

# Generate changelog for latest tag only
changelog-latest:
    @echo "📋 Generating changelog for latest tag..."
    @if command -v git-cliff >/dev/null 2>&1; then \
        git-cliff --latest; \
    else \
        echo "ERROR: git-cliff not installed. Run: cargo install git-cliff"; \
        exit 1; \
    fi

# Append unreleased changes to existing CHANGELOG.md (for releases)
changelog-append:
    @echo "📝 Appending unreleased changes to CHANGELOG.md..."
    @if command -v git-cliff >/dev/null 2>&1; then \
        git-cliff --unreleased --prepend CHANGELOG.md; \
        echo "✅ Changelog updated with unreleased changes"; \
    else \
        echo "ERROR: git-cliff not installed. Run: cargo install git-cliff"; \
        exit 1; \
    fi

# ============================================================================
# Dead Code Detection (Issue #284)
# ============================================================================
# Detect unused dependencies, dead code, and unused imports/variables.
# Uses cargo-udeps and clippy dead_code lints.

# Run dead code detection (local check)
dead-code:
    @echo "🔍 Running dead code detection..."
    @bash scripts/dead-code-check.sh check

# Generate dead code baseline
dead-code-baseline:
    @echo "📝 Generating dead code baseline..."
    @bash scripts/dead-code-check.sh baseline

# Generate dead code report (JSON)
dead-code-report:
    @echo "📊 Generating dead code report..."
    @bash scripts/dead-code-check.sh report

# Run dead code detection in strict mode (fail on any increase)
dead-code-strict:
    @echo "🔍 Running dead code detection (strict mode)..."
    @DEAD_CODE_STRICT=true bash scripts/dead-code-check.sh check

# CI gate: fail if dead code exceeds baseline
ci-dead-code:
    @echo "🔍 Checking dead code baseline..."
    @bash scripts/dead-code-check.sh check

# ============================================================================
# CI Gate Execution with Receipt Generation (Issue #210)
# ============================================================================

# CI gate: check unlinked-item compliance
ci-check-todos:
    @bash ci/check_todos.sh

# Fast merge gate with receipt generation
ci-gate-with-receipts:
    @echo "🚪 Running fast merge gate with receipts..."
    @mkdir -p .receipts/$(date +%Y%m%d)
    @RECEIPT_DIR=".receipts/$(date +%Y%m%d)" bash -c '\
        ./scripts/execute-gate.sh workflow-audit --receipt-dir "$RECEIPT_DIR" && \
        ./scripts/execute-gate.sh no-nested-lock --receipt-dir "$RECEIPT_DIR" && \
        ./scripts/execute-gate.sh format --receipt-dir "$RECEIPT_DIR" && \
        ./scripts/execute-gate.sh clippy-lib --receipt-dir "$RECEIPT_DIR" && \
        ./scripts/execute-gate.sh test-lib --receipt-dir "$RECEIPT_DIR" && \
        ./scripts/execute-gate.sh policy --receipt-dir "$RECEIPT_DIR" && \
        ./scripts/execute-gate.sh lsp-definition --receipt-dir "$RECEIPT_DIR" \
    '
    @echo "✅ Merge gate passed with receipts!"
    @echo "📁 Receipts: .receipts/$(date +%Y%m%d)/"

# Gate execution for individual gate (with receipt)
gate-execute gate_id:
    @./scripts/execute-gate.sh {{gate_id}} --receipt-dir .receipts/$(date +%Y%m%d)

# Show gate registry
gate-list:
    @python3 scripts/list-gates.py

# ============================================================================
# Release Gate (Slice C: Release candidate validation)
# ============================================================================

# Release build (locked, optimized)
release-build:
    @echo "Building release binary..."
    cargo build -p perl-lsp --release --locked
    @echo "Release build complete: target/release/perl-lsp"

# Version sync check (Slice B: single source of version truth)
version-check:
    @echo "Checking version sync..."
    @bash scripts/check-version-sync.sh

# Release gate: full validation for release candidates (~10 min)
# Composes: ci-gate + release-specific checks
release-gate: ci-gate release-build sbom-verify version-check
    @echo "=============================================="
    @echo "  RELEASE GATE PASSED"
    @echo "=============================================="

# ============================================================================
# LSP Test Tiering (Slice D: tiered test execution)
# ============================================================================

# Tier A: fast smoke tests for perl-lsp (<30s)
# Run on every PR for quick feedback
lsp-tier-a:
    @echo "Running LSP Tier A (smoke tests)..."
    cargo test -p perl-lsp --test cli_smoke --test lsp_capabilities_snapshot --test lsp_capabilities_contract --test lsp_protocol_tests --locked -- --test-threads=1
    @echo "LSP Tier A passed"

# Tier B: core behavior tests for perl-lsp (~2-5 min)
# Run at merge gate for thorough validation
lsp-tier-b: lsp-tier-a
    @echo "Running LSP Tier B (core behavior)..."
    env RUST_TEST_THREADS=2 cargo test -p perl-lsp \
        --test semantic_definition \
        --test lsp_completion_tests \
        --test lsp_unhappy_paths \
        --test lsp_code_actions_test \
        --test execute_command_security_tests \
        --test lsp_behavioral_tests \
        --test lsp_workspace_index_e2e \
        --locked -- --test-threads=2
    @echo "LSP Tier B passed"

# Tier C: full suite (nightly, all integration tests)
lsp-tier-c:
    @echo "Running LSP Tier C (full suite)..."
    env RUST_TEST_THREADS=2 cargo test -p perl-lsp --locked -- --test-threads=2
    @echo "LSP Tier C passed"
