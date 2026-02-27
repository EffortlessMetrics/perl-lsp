# Test Infrastructure Guide

> This guide follows the **[Diataxis framework](https://diataxis.fr/)** for comprehensive technical documentation:
> - **Tutorial sections**: Hands-on learning with test execution examples
> - **How-to sections**: Step-by-step guidance for specific testing scenarios
> - **Reference sections**: Complete technical specifications and test categories
> - **Explanation sections**: Design concepts and testing philosophy

## Table of Contents

1. [Test Categories](#test-categories)
2. [Test Discovery Protocol](#test-discovery-protocol)
3. [Timeout Strategy](#timeout-strategy)
4. [Nextest Configuration](#nextest-configuration)
5. [Known Flaky Tests](#known-flaky-tests)
6. [Environment Variables](#environment-variables)
7. [Running Tests Locally vs CI](#running-tests-locally-vs-ci)
8. [Test Quality Assurance](#test-quality-assurance)

---

## Test Categories

### Overview (*Diataxis: Reference*)

The Perl LSP project uses a comprehensive multi-tiered testing strategy with ~720 baseline tests across unit, integration, property-based, and E2E categories:

```
┌─────────────────────────────────────────────────────────┐
│                 Test Pyramid Structure                  │
├─────────────────────────────────────────────────────────┤
│  E2E Tests (~50)           │ Full workflow validation   │
│  Integration Tests (~200)  │ LSP protocol compliance    │
│  Property Tests (~100)     │ Invariant verification     │
│  Unit Tests (~370)         │ Component isolation        │
└─────────────────────────────────────────────────────────┘
```

**Baseline Test Count**: 720 tests (as of 2025-12-31)
- **perl-lexer**: 12 tests
- **perl-parser**: 324 tests (lib tests only)
- **perl-lsp**: 37 tests (lib tests only)
- **perl-dap**: 9 tests
- **Integration tests**: ~338+ additional tests in test files

**5% Drop Threshold**: If test count falls below 684 tests (720 × 0.95), investigate for accidentally disabled or deleted tests.

### 1. Unit Tests (*Diataxis: Tutorial*)

**Purpose**: Fast, isolated component testing with millisecond execution times.

**Location**: `crates/*/src/` (inline `#[cfg(test)]` modules) and `crates/*/tests/*_test.rs`

**Examples**:
```bash
# Run all unit tests across workspace
cargo test --workspace --lib

# Run parser unit tests only
cargo test -p perl-parser --lib

# Run specific unit test module
cargo test -p perl-parser semantic::tests
```

**Characteristics**:
- **Execution time**: <1ms per test typically
- **Parallelization**: Safe to run with unlimited threads
- **Dependencies**: No external services or I/O
- **Coverage**: Core parsing, semantic analysis, utility functions

**Key Test Modules**:
- Parser unit tests: `/crates/perl-parser/src/lib.rs` (semantic analysis, AST validation)
- Lexer unit tests: `/crates/perl-lexer/src/lib.rs` (tokenization, Unicode handling)
- LSP unit tests: `/crates/perl-lsp/src/lib.rs` (protocol handling, state management)

### 2. Integration Tests (*Diataxis: Tutorial*)

**Purpose**: Test component interactions, LSP protocol compliance, and cross-module behavior.

**Location**: `crates/perl-lsp/tests/lsp_*.rs` and `crates/perl-parser/tests/*_tests.rs`

**Examples**:
```bash
# Run all LSP integration tests (with adaptive threading)
RUST_TEST_THREADS=2 cargo test -p perl-lsp -- --test-threads=2

# Run specific integration test suite
cargo test -p perl-lsp --test lsp_comprehensive_e2e_test

# Run parser integration tests
cargo test -p perl-parser --test substitution_fixed_tests
```

**Characteristics**:
- **Execution time**: 10ms-500ms per test
- **Parallelization**: Requires thread management (RUST_TEST_THREADS=2 recommended)
- **Dependencies**: LSP server process, file I/O, workspace indexing
- **Coverage**: LSP features, cross-file navigation, refactoring operations

**Key Test Files**:
- `lsp_comprehensive_e2e_test.rs`: Full LSP workflow validation
- `lsp_behavioral_tests.rs`: LSP feature behavior (0.31s with RUST_TEST_THREADS=2)
- `lsp_full_coverage_user_stories.rs`: User story validation (0.32s with RUST_TEST_THREADS=2)
- `semantic_definition.rs`: Semantic-aware go-to-definition tests

### 3. Property-Based Tests (*Diataxis: Tutorial*)

**Purpose**: Validate invariants across randomly generated inputs using proptest framework.

**Location**: `crates/*/tests/prop_*.rs` and `crates/*/tests/fuzz_*.rs`

**Examples**:
```bash
# Run all property-based tests
cargo test -p perl-parser prop_

# Run specific property test suite
cargo test -p perl-parser --test prop_whitespace_idempotence

# Run fuzzing tests with bounded iterations
cargo test -p perl-parser --test fuzz_quote_parser_comprehensive
```

**Characteristics**:
- **Execution time**: 100ms-5s per test (depends on iterations)
- **Parallelization**: CPU-intensive, benefits from multiple cores
- **Dependencies**: Proptest crate, regression file management
- **Coverage**: Parser invariants, Unicode handling, incremental parsing

**Key Test Categories**:
- **Parser Fuzzing**: `fuzz_quote_parser_*.rs`, `fuzz_incremental_parsing.rs`
- **Position Tracking**: `prop_position_utf16.rs` (UTF-16/UTF-8 conversion)
- **Whitespace Handling**: `prop_whitespace*.rs` (parsing idempotence)
- **Quote-Like Operators**: `prop_quote_like.rs`, `prop_qw.rs`

**Proptest Configuration**:
```rust
// Typical configuration in prop_*.rs files
proptest! {
    #![proptest_config(ProptestConfig {
        cases: 100,           // Number of test cases
        max_shrink_iters: 1000,
        timeout: 5000,        // 5 second timeout
        .. ProptestConfig::default()
    })]

    #[test]
    fn test_invariant(input in strategy()) {
        // Property verification
    }
}
```

### 4. End-to-End (E2E) Tests (*Diataxis: Tutorial*)

**Purpose**: Full workflow validation including LSP server lifecycle, client communication, and workspace operations.

**Location**: `crates/perl-lsp/tests/lsp_comprehensive_e2e_test.rs`

**Examples**:
```bash
# Run comprehensive E2E test (maximum reliability)
RUST_TEST_THREADS=1 cargo test --test lsp_comprehensive_e2e_test

# Run with adaptive threading (faster)
RUST_TEST_THREADS=2 cargo test -p perl-lsp --test lsp_comprehensive_e2e_test -- --test-threads=2

# Run with debugging output
RUST_LOG=debug RUST_TEST_THREADS=1 cargo test -p perl-lsp --test lsp_comprehensive_e2e_test -- --nocapture
```

**Characteristics**:
- **Execution time**: 5s-60s per test suite
- **Parallelization**: Serial execution recommended (RUST_TEST_THREADS=1)
- **Dependencies**: Full LSP server, JSON-RPC protocol, workspace files
- **Coverage**: Complete LSP workflows, initialization, shutdown, error handling

**Key Workflows Tested**:
1. **Initialization**: Server startup, capability negotiation, workspace indexing
2. **Navigation**: Go-to-definition, find references, call hierarchy
3. **Completion**: Context-aware suggestions, documentation, detail formatting
4. **Refactoring**: Rename, extract variable/subroutine, import optimization
5. **Diagnostics**: Syntax errors, semantic warnings, code actions

---

## Test Discovery Protocol

### Baseline Verification (*Diataxis: Reference*)

**Current Baseline**: 720 tests (workspace lib tests: 382 tests)

**Verification Commands**:
```bash
# Quick baseline check (lib tests only)
cargo test --workspace --lib 2>&1 | grep "test result:"

# Full test discovery (all targets)
cargo test --workspace --no-run 2>&1 | grep -E "(test|running)" | wc -l

# Per-crate breakdown
cargo test --workspace --lib -- --list | grep -c "^test"
```

**Expected Output** (workspace lib tests):
```
running 12 tests    # perl-lexer
running 37 tests    # perl-lsp
running 9 tests     # perl-dap
running 324 tests   # perl-parser
```

### Test Count Monitoring (*Diataxis: How-to*)

**5% Drop Threshold**: Investigate if total test count falls below 684 tests.

**Automated Monitoring**:
```bash
#!/bin/bash
# Save as scripts/check-test-count.sh
BASELINE=720
THRESHOLD=$(echo "$BASELINE * 0.95" | bc | cut -d. -f1)

CURRENT=$(cargo test --workspace --lib -- --list 2>/dev/null | grep -c "^test")

if [ "$CURRENT" -lt "$THRESHOLD" ]; then
    echo "::error::Test count dropped below threshold: $CURRENT < $THRESHOLD (5% drop from $BASELINE)"
    exit 1
else
    echo "Test count OK: $CURRENT tests (baseline: $BASELINE, threshold: $THRESHOLD)"
fi
```

**CI Integration**:
```yaml
# Add to .github/workflows/ci.yml
- name: Verify test count (5% drop threshold)
  run: |
    BASELINE=720
    THRESHOLD=$((BASELINE * 95 / 100))
    CURRENT=$(cargo test --workspace --lib -- --list 2>/dev/null | grep -c "^test" || echo 0)
    if [ "$CURRENT" -lt "$THRESHOLD" ]; then
      echo "::error::Test count dropped: $CURRENT < $THRESHOLD"
      exit 1
    fi
```

---

## Timeout Strategy

### Adaptive Timeout Architecture (*Diataxis: Explanation*)

The LSP server implements **adaptive timeout scaling** based on thread count detection (PR #140):

```
┌─────────────────────────────────────────────────────────┐
│           Adaptive Timeout Decision Matrix              │
├─────────────────────────────────────────────────────────┤
│ Thread Count │ Timeout      │ Sleep Multiplier │ Use   │
├──────────────┼──────────────┼──────────────────┼───────┤
│ ≤2 threads   │ 15s (500ms*) │ 3x               │ CI    │
│ ≤4 threads   │ 10s (300ms*) │ 2x               │ Dev   │
│ 5-8 threads  │ 7.5s (200ms*)│ 1.5x             │ Local │
│ >8 threads   │ 5s (200ms*)  │ 1x               │ Full  │
└──────────────┴──────────────┴──────────────────┴───────┘

* LSP harness millisecond-precision timeouts (see get_adaptive_timeout)
```

### Multi-Tier Timeout System (*Diataxis: Reference*)

#### 1. LSP Harness Timeouts (Millisecond Precision)

**Purpose**: Fine-grained timeout control for LSP JSON-RPC message handling.

**Implementation**:
```rust
/// Get adaptive timeout based on RUST_TEST_THREADS environment variable
fn get_adaptive_timeout(&self) -> Duration {
    let thread_count = std::env::var("RUST_TEST_THREADS")
        .ok()
        .and_then(|s| s.parse::<usize>().ok())
        .unwrap_or(4);

    match thread_count {
        0..=2 => Duration::from_millis(500), // High contention
        3..=4 => Duration::from_millis(300), // Medium contention
        _ => Duration::from_millis(200),     // Low contention
    }
}
```

**Use Cases**:
- JSON-RPC request/response cycles
- LSP initialization handshake
- Rapid message exchange in behavioral tests

#### 2. Comprehensive Test Timeouts (Second Precision)

**Purpose**: Broader timeout scaling for complete test execution.

**Scaling Formula**:
```rust
pub fn adaptive_timeout() -> Duration {
    let base_timeout = Duration::from_secs(5);
    let thread_count = max_concurrent_threads();

    match thread_count {
        0..=2 => base_timeout * 3,   // 15s for heavily constrained
        3..=4 => base_timeout * 2,   // 10s for moderately constrained
        5..=8 => base_timeout * 3/2, // 7.5s for lightly constrained
        _ => base_timeout,           // 5s for unconstrained
    }
}
```

**Performance Results** (PR #140):
```
Before Adaptive Timeouts          After Adaptive Timeouts (RUST_TEST_THREADS=2)
─────────────────────────────────  ─────────────────────────────────────────────
lsp_behavioral_tests:            0.31s
lsp_user_stories:                0.32s
Individual workspace tests:      0.26s
Overall test suite:              <10s
CI reliability:                  100% pass rate
```

#### 3. Optimized Idle Detection

**Before PR #140**: 1000ms polling cycles
**After PR #140**: 200ms polling cycles (**5x improvement**)

```rust
pub fn wait_for_idle(&self) {
    let sleep_duration = Duration::from_millis(200); // Optimized from 1000ms
    let thread_count = max_concurrent_threads();

    let multiplier = match thread_count {
        0..=2 => 3, // CI environments need longer stabilization
        3..=4 => 2,
        _ => 1,
    };

    std::thread::sleep(sleep_duration * multiplier);
}
```

### Timeout Configuration Best Practices (*Diataxis: How-to*)

**For CI Environments**:
```bash
# GitHub Actions, GitLab CI, etc.
RUST_TEST_THREADS=2 cargo test -p perl-lsp -- --test-threads=2
```

**For Local Development**:
```bash
# Fast iteration (4 threads)
RUST_TEST_THREADS=4 cargo test -p perl-lsp

# Maximum reliability (serial)
RUST_TEST_THREADS=1 cargo test -p perl-lsp --test specific_test -- --nocapture
```

**For High-Performance Workstations**:
```bash
# Let system auto-detect (8+ threads)
cargo test --workspace

# Or explicitly set high thread count
RUST_TEST_THREADS=8 cargo test --workspace
```

---

## Nextest Configuration

### Overview (*Diataxis: Reference*)

The project uses **cargo-nextest** for enhanced test execution with retry support, slow test detection, and thread management.

**Configuration File**: `.cargo/nextest.toml`

**Installation**:
```bash
cargo install cargo-nextest
```

### Profiles (*Diataxis: Reference*)

#### 1. Default Profile (Local Development)

```toml
[profile.default]
retries = 0  # No retries for fast feedback
```

**Usage**:
```bash
cargo nextest run --workspace
```

#### 2. CI Profile (Continuous Integration)

```toml
[profile.ci]
# Retry support for flaky tests
retries = 2
fail-fast = false

# Slow test detection
slow-timeout = { period = "60s", terminate-after = 2 }
leak-timeout = "10s"

[profile.ci.junit]
path = "target/nextest/ci/junit.xml"
```

**Usage**:
```bash
cargo nextest run --profile ci --workspace
```

**Features**:
- **Retries**: Up to 2 retries for flaky tests
- **Slow Timeout**: Warn if tests exceed 60s, terminate after 2 occurrences
- **JUnit Reports**: Generate XML reports for CI integration

#### 3. LSP Integration Test Overrides

```toml
[profile.ci.overrides]
# LSP tests need more time in CI
filter = 'package(perl-lsp)'
slow-timeout = { period = "120s", terminate-after = 3 }
threads-required = 2
```

**Why 120s?**: Accounts for:
1. LSP server initialization (5-10s)
2. Workspace indexing (10-30s for large codebases)
3. JSON-RPC message exchange (5-10s)
4. Graceful shutdown (2-5s)
5. CI environment overhead (2-3x local execution time)

#### 4. Cancellation Test Overrides

```toml
[[profile.ci.overrides]]
filter = 'test(lsp_cancel)'
threads-required = 1
retries = 3
```

**Rationale**: Cancellation tests require careful thread management to validate race condition handling.

### Flaky Test Retry Configuration (*Diataxis: Reference*)

```toml
# Known flaky BrokenPipe tests get extra retries
[[profile.ci.overrides]]
filter = 'test(lsp_document_symbols) | test(lsp_document_links) | test(lsp_encoding)'
retries = 3
```

**Why Extra Retries?**: These tests occasionally experience BrokenPipe errors during LSP server teardown in constrained CI environments (see [Known Flaky Tests](#known-flaky-tests)).

### Local Fast Profile

```toml
[profile.local-fast]
test-threads = 4
retries = 0
```

**Usage**:
```bash
cargo nextest run --profile local-fast --workspace
```

**Purpose**: Rapid iteration during development with balanced parallelization.

---

## Known Flaky Tests

### Overview (*Diataxis: Explanation*)

Most historical "BrokenPipe" test failures were environmental/timing issues, now fully resolved by:
1. Adaptive threading configuration (RUST_TEST_THREADS=2)
2. Enhanced LSP harness with graceful shutdown handling
3. Proper connection lifecycle management

**Current Status** (run `bash scripts/ignored-test-count.sh` for live counts):
- **Tracked test debt**: BUG=0, MANUAL=1 (utility only)
- **Non-default lanes**: Feature-gated tests run with `--all-features` or specific feature flags
- **Baseline**: `scripts/.ignored-baseline`

### Known Flaky Test Categories (*Diataxis: Reference*)

#### 1. LSP Document Symbols (`lsp_document_symbols_test.rs`)

**Symptom**: Occasional BrokenPipe errors during teardown

**Root Cause**: Race condition between:
1. Test requesting document symbols
2. LSP server processing workspace index
3. Test harness initiating shutdown before response sent

**Mitigation**:
```toml
[[profile.ci.overrides]]
filter = 'test(lsp_document_symbols)'
retries = 3
slow-timeout = { period = "120s", terminate-after = 3 }
```

**Workaround**:
```bash
# Run with serial execution for 100% reliability
RUST_TEST_THREADS=1 cargo test -p perl-lsp --test lsp_document_symbols_test
```

#### 2. LSP Document Links (`lsp_document_links_test.rs`)

**Symptom**: BrokenPipe during cross-file navigation analysis

**Root Cause**: Workspace indexing still in progress when test requests document links

**Mitigation**:
```toml
[[profile.ci.overrides]]
filter = 'test(lsp_document_links)'
retries = 3
```

**Workaround**:
```bash
# Use enhanced wait_for_idle with 3x multiplier
RUST_TEST_THREADS=2 cargo test -p perl-lsp --test lsp_document_links_test -- --test-threads=2
```

#### 3. LSP Encoding Edge Cases (`lsp_encoding_edge_cases.rs`)

**Symptom**: Intermittent failures with UTF-16/UTF-8 position conversion

**Root Cause**: Timing-sensitive Unicode boundary calculations during rapid message exchange

**Current Status**: Tests pass reliably with RUST_TEST_THREADS=2. `#[ignore]` annotations removed during the Wave C cleanup (PR #261).

**Mitigation**:
```toml
[[profile.ci.overrides]]
filter = 'test(lsp_encoding)'
retries = 3
```

**Verification**:
```bash
# Remove #[ignore] and validate
cargo test -p perl-lsp --test lsp_encoding_edge_cases -- --nocapture
```

### BrokenPipe Error Handling (*Diataxis: How-to*)

**Understanding BrokenPipe Errors**:
```rust
/// Connection closed - BrokenPipe or similar transport termination
pub enum LspErrorKind {
    ConnectionClosed, // Maps to ErrorCode::ConnectionClosed (-32050)
    // ...
}

/// BrokenPipe → CONNECTION_CLOSED (-32050)
impl From<io::Error> for LspError {
    fn from(e: io::Error) -> Self {
        if e.kind() == io::ErrorKind::BrokenPipe {
            LspError::connection_closed("BrokenPipe during communication")
        } else {
            LspError::transport_failed(e.to_string())
        }
    }
}
```

**Graceful Shutdown Pattern**:
```rust
// Ignore write errors during teardown - BrokenPipe is expected
let _ = writer.write_all(shutdown_request.as_bytes());
let _ = writer.flush();

// Allow time for graceful shutdown before forceful termination
std::thread::sleep(Duration::from_millis(100));
```

**Testing BrokenPipe Resilience**:
```bash
# Torture test for connection handling
cargo test -p perl-lsp --test lsp_init_torture_test -- --nocapture
```

---

## Environment Variables

### RUST_TEST_THREADS (*Diataxis: Reference*)

**Enhancement in PR #140**: Adaptive threading for fast CI test execution.

**Purpose**: Control test parallelism and trigger adaptive timeout scaling.

**Values**:
- `RUST_TEST_THREADS=1`: Serial execution (maximum reliability)
- `RUST_TEST_THREADS=2`: CI optimal (recommended for GitHub Actions)
- `RUST_TEST_THREADS=4`: Balanced development (default if unset)
- `RUST_TEST_THREADS=8+`: High-performance workstations

**Examples**:
```bash
# CI configuration (GitHub Actions)
RUST_TEST_THREADS=2 cargo test -p perl-lsp -- --test-threads=2

# Development (fast iteration)
RUST_TEST_THREADS=4 cargo test --workspace

# Debugging (serial, full output)
RUST_TEST_THREADS=1 cargo test -p perl-lsp --test specific_test -- --nocapture
```

**Implementation**:
```rust
pub fn max_concurrent_threads() -> usize {
    std::env::var("RUST_TEST_THREADS")
        .ok()
        .and_then(|s| s.parse::<usize>().ok())
        .unwrap_or_else(|| {
            std::thread::available_parallelism().map(|n| n.get()).unwrap_or(8)
        })
        .max(1)
}
```

### LSP_TEST_FALLBACKS (*Diataxis: Reference*)

**NEW in v0.8.8**: Fast testing mode reducing timeouts by 75% (2000ms → 500ms).

**Purpose**: Enable rapid testing with mock responses for CI/development iteration.

**Values**:
- `LSP_TEST_FALLBACKS=1`: Enabled (fast mode)
- Unset: Disabled (full validation mode)

**Examples**:
```bash
# Fast workspace validation (<10s total)
LSP_TEST_FALLBACKS=1 cargo test --workspace

# Combine with adaptive threading
RUST_TEST_THREADS=2 LSP_TEST_FALLBACKS=1 cargo test -p perl-lsp -- --test-threads=2

# Quick build verification
LSP_TEST_FALLBACKS=1 cargo check --workspace
```

**Behavior**:
```rust
let use_fallback = std::env::var("LSP_TEST_FALLBACKS").is_ok();

if use_fallback {
    // Fast mode: 500ms timeout, mock responses
    Duration::from_millis(500)
} else {
    // Full mode: Adaptive timeout (5s-15s), real LSP server
    adaptive_timeout()
}
```

**Use Cases**:
- Pre-commit checks (fast feedback)
- CI gate validation (2-5 min total)
- Development iteration (rapid test cycles)

### RUN_REAL_WORLD (*Diataxis: Reference*)

**Purpose**: Enable expensive real-world corpus testing (disabled by default).

**Values**:
- `RUN_REAL_WORLD=1`: Run real-world corpus tests
- Unset: Skip real-world tests (default)

**Examples**:
```bash
# Run comprehensive corpus validation (15-30 min)
RUN_REAL_WORLD=1 cargo test -p perl-corpus

# Combine with threading for parallel corpus parsing
RUN_REAL_WORLD=1 RUST_TEST_THREADS=8 cargo test -p perl-corpus
```

**Corpus Size**:
- ~10,000+ Perl files from CPAN modules
- ~5MB+ of Perl source code
- Edge case coverage: heredocs, Unicode, operator precedence

### CI (*Diataxis: Reference*)

**Purpose**: Detect CI environment and adjust test behavior.

**Values**:
- `CI=true`: Running in CI (GitHub Actions, GitLab CI)
- Unset: Local development

**Automatic Detection**:
```rust
let is_ci = std::env::var("CI").is_ok();

if is_ci {
    // CI mode: Conservative timeouts, retry support
    adaptive_timeout() * 2
} else {
    // Local mode: Aggressive timeouts, fast feedback
    adaptive_timeout()
}
```

**GitHub Actions Automatic Setting**:
```yaml
# Automatically set by GitHub Actions
env:
  CI: true
  RUST_TEST_THREADS: 2
```

### LSP_TEST_ECHO_STDERR (*Diataxis: Reference*)

**Purpose**: Echo LSP server stderr to test output for debugging.

**Values**:
- `LSP_TEST_ECHO_STDERR=1`: Echo stderr
- Unset: Silent stderr

**Examples**:
```bash
# Debug LSP server communication
LSP_TEST_ECHO_STDERR=1 RUST_TEST_THREADS=2 cargo test -p perl-lsp -- --nocapture

# Combine with RUST_LOG for comprehensive debugging
RUST_LOG=debug LSP_TEST_ECHO_STDERR=1 cargo test -p perl-lsp --test specific_test -- --nocapture
```

### RUST_LOG (*Diataxis: Reference*)

**Purpose**: Control tracing/logging output level.

**Values**:
- `RUST_LOG=error`: Errors only
- `RUST_LOG=warn`: Warnings and errors
- `RUST_LOG=info`: Informational messages
- `RUST_LOG=debug`: Detailed debugging
- `RUST_LOG=trace`: Verbose trace logging

**Examples**:
```bash
# Debug test failures
RUST_LOG=debug cargo test -p perl-lsp --test failing_test -- --nocapture

# Trace LSP protocol messages
RUST_LOG=trace cargo test -p perl-lsp --test lsp_comprehensive_e2e_test -- --nocapture

# Module-specific logging
RUST_LOG=perl_parser::semantic=debug cargo test -p perl-parser
```

---

## Running Tests Locally vs CI

### Local Development (*Diataxis: How-to*)

#### Quick Iteration Workflow

```bash
# 1. Fast unit tests (milliseconds)
cargo test --workspace --lib

# 2. Specific component testing
cargo test -p perl-parser --lib

# 3. Integration tests with fast mode
LSP_TEST_FALLBACKS=1 RUST_TEST_THREADS=4 cargo test -p perl-lsp

# 4. Full validation before commit
RUST_TEST_THREADS=2 cargo test -p perl-lsp -- --test-threads=2
```

#### Debugging Specific Test

```bash
# Serial execution with full output
RUST_TEST_THREADS=1 cargo test -p perl-lsp --test lsp_comprehensive_e2e_test \
    -- test_workspace_symbol_search --nocapture

# With debugging logs
RUST_LOG=debug RUST_TEST_THREADS=1 cargo test -p perl-lsp \
    --test semantic_definition -- --nocapture

# With LSP server stderr
LSP_TEST_ECHO_STDERR=1 RUST_LOG=debug cargo test -p perl-lsp \
    --test failing_test -- --nocapture
```

#### Performance Benchmarking

```bash
# Compare threading configurations
for threads in 1 2 4 8; do
    echo "Testing with $threads threads:"
    time RUST_TEST_THREADS=$threads cargo test -p perl-lsp
done

# Profile test execution
cargo nextest run --profile local-fast --workspace
```

### CI Environment (*Diataxis: How-to*)

#### GitHub Actions Configuration

**Recommended Setup**:
```yaml
name: CI

on:
  pull_request:
    branches: [ master ]
  push:
    branches: [ master ]

jobs:
  test:
    runs-on: ubuntu-latest
    env:
      RUST_TEST_THREADS: 2
      RUST_BACKTRACE: full
      CARGO_NET_RETRY: 4

    steps:
      # Supply-chain security: Pin third-party actions by commit SHA
      # Keep version tags in comments for readability; update SHAs on a schedule
      - uses: actions/checkout@<COMMIT_SHA> # v4
      - uses: dtolnay/rust-toolchain@<COMMIT_SHA> # stable
        with:
          toolchain: 1.90.0

      - uses: Swatinem/rust-cache@<COMMIT_SHA> # v2
        with:
          cache-on-failure: true

      # Fast gate: Unit tests (2-3 min)
      - name: Core tests
        run: cargo test --locked --workspace --lib

      # LSP integration tests (5-10 min)
      - name: LSP tests
        run: |
          RUST_TEST_THREADS=2 cargo test --locked -p perl-lsp -- --test-threads=2

      # Comprehensive E2E test (10-15 min)
      - name: E2E test
        run: |
          RUST_TEST_THREADS=2 cargo test -p perl-lsp \
            --test lsp_comprehensive_e2e_test -- --test-threads=2
```

#### Nextest Integration

```yaml
      - name: Install nextest
        run: cargo install cargo-nextest --locked

      - name: Run tests with nextest
        run: |
          cargo nextest run --profile ci --workspace \
            --junit target/nextest/ci/junit.xml

      - name: Publish test results
        uses: EnricoMi/publish-unit-test-result-action@<COMMIT_SHA> # v2
        if: always()
        with:
          junit_files: target/nextest/ci/junit.xml
```

#### GitLab CI Configuration

```yaml
test:
  stage: test
  image: rust:1.90
  variables:
    RUST_TEST_THREADS: "2"
    CARGO_NET_RETRY: "4"
  script:
    - cargo test --locked --workspace --lib
    - RUST_TEST_THREADS=2 cargo test -p perl-lsp -- --test-threads=2
  artifacts:
    reports:
      junit: target/nextest/ci/junit.xml
```

### Justfile Recipes (*Diataxis: How-to*)

The project provides **justfile** recipes for standardized test execution:

```bash
# Install just
cargo install just

# Fast merge gate (~2-5 min) - REQUIRED for all merges
just ci-gate

# Full CI pipeline (~10-20 min) - RECOMMENDED for large changes
just ci-full

# Individual gates
just ci-format          # Format check
just ci-clippy-lib      # Clippy (libraries only)
just ci-test-lib        # Library tests
just ci-lsp-def         # LSP semantic definition tests

# Development commands
just build              # Build all workspace crates
just test               # Run all tests
just fmt                # Format code
just health             # Show codebase health metrics
```

**Fast Merge Gate** (`just ci-gate`):
```bash
# Runs in 2-5 minutes
cargo fmt --check --all
cargo clippy --workspace --lib --locked -- -D warnings -A missing_docs
cargo test --workspace --lib --locked
RUSTC_WRAPPER="" RUST_TEST_THREADS=1 CARGO_BUILD_JOBS=1 \
    cargo test -p perl-lsp --test semantic_definition -- --test-threads=1
```

**Full CI Pipeline** (`just ci-full`):
```bash
# Runs in 10-20 minutes
cargo fmt --check --all
cargo clippy --workspace --all-targets -- -D warnings -A missing_docs
cargo test --workspace --lib --bins
RUST_TEST_THREADS=2 cargo test -p perl-lsp --test lsp_comprehensive_e2e_test -- --test-threads=2
cargo doc -p perl-parser -p perl-lsp --no-deps
```

---

## Test Quality Assurance

### Mutation Testing (*Diataxis: Tutorial*)

**Purpose**: Validate test effectiveness by introducing code mutations and ensuring tests fail.

**Installation**:
```bash
cargo install cargo-mutants
```

**Examples**:
```bash
# Run mutation tests on parser
cargo mutants --package perl-parser --timeout 300

# Specific mutation test suite
cargo test -p perl-parser --test mutation_hardening_tests

# With adaptive threading
RUST_TEST_THREADS=2 cargo test -p perl-parser --test mutation_hardening_tests
```

**Quality Metrics**:
- **Mutation Score**: 87% (improved from ~70% in PR #153)
- **Test Suites**: 7 mutation hardening test files
- **Coverage**: 147 tests for comprehensive edge case coverage

**Key Test Files**:
- `mutation_hardening_tests.rs`: Core parser mutations (60%+ score improvement)
- `quote_parser_mutation_hardening.rs`: Quote operator edge cases
- `cancellation_atomic_operations_hardening.rs`: Concurrency mutations
- `documentation_validation_mutation_hardening.rs`: API documentation mutations

### Fuzz Testing (*Diataxis: Tutorial*)

**Purpose**: Property-based testing with crash detection and AST invariant validation.

**Examples**:
```bash
# Comprehensive quote parser fuzzing
cargo test -p perl-parser --test fuzz_quote_parser_comprehensive

# Incremental parsing stress testing
cargo test -p perl-parser --test fuzz_incremental_parsing

# Substitution operator fuzzing
cargo test -p perl-parser --test substitution_fuzz_tests
```

**Quality Metrics**:
- **Test Suites**: 12 fuzz testing files
- **Coverage**: Property-based testing, crash detection, AST invariants
- **Regression Files**: `.proptest-regressions/` directories track discovered issues

**Key Test Files**:
- `fuzz_quote_parser_*.rs`: Quote operator fuzzing with delimiter handling
- `fuzz_incremental_parsing.rs`: Incremental parser stress testing
- `fuzz_documentation_infrastructure_pr159.rs`: Documentation infrastructure fuzzing

### Test Coverage Tracking (*Diataxis: How-to*)

**Installation**:
```bash
cargo install cargo-tarpaulin
```

**Generate Coverage Report**:
```bash
# Full workspace coverage
cargo tarpaulin --workspace --out Html --output-dir target/coverage

# LSP-specific coverage
cargo tarpaulin -p perl-lsp --out Lcov --output-dir target/coverage/lsp

# Exclude property tests (reduce noise)
cargo tarpaulin --workspace --exclude-files '**/prop_*.rs' --out Html
```

**Expected Coverage**:
- **Parser core**: 85-90% line coverage
- **LSP providers**: 80-85% line coverage
- **Integration tests**: 70-75% feature coverage
- **Property tests**: 95%+ invariant coverage

### Test Hygiene Metrics (*Diataxis: Reference*)

**Health Scoreboard** (`just health`):
```bash
📊 Codebase Health Scoreboard
==============================

📝 Ignored Tests by Crate:
  perl-parser: 45
  perl-lsp:    720 (87% BrokenPipe candidates for removal)
  perl-lexer:  3
  perl-dap:    0

⚠️  Unwrap/Expect Count (potential panic sites):
  .unwrap():  234
  .expect(:   89

🖨️  Debug Print Count (should use tracing):
  println!:   12
  eprintln!:  45
```

**Detailed Metrics** (`just health-detail`):
```bash
🔴 Top 10 files with most .unwrap() calls:
  crates/perl-parser/src/lsp_server.rs: 67
  crates/perl-parser/src/semantic.rs: 23
  ...

🟡 Top 10 files with most eprintln! calls:
  crates/perl-lsp/tests/common/mod.rs: 12
  ...
```

### Test Documentation Standards (*Diataxis: Reference*)

**Acceptance Criteria Validation**:
```bash
# Validate documentation acceptance criteria
cargo test -p perl-parser --test missing_docs_ac_tests

# Detailed validation output
cargo test -p perl-parser --test missing_docs_ac_tests -- --nocapture
```

**12 Acceptance Criteria**:
1. ✅ Missing docs warning compilation enabled
2. ⏳ Public functions documentation presence (Phase 1 target)
3. ⏳ Public structs documentation presence (Phase 1 target)
4. ⏳ Performance documentation presence (Phase 1 target)
5. ⏳ Module-level documentation presence (Phase 1 target)
6. ⏳ Usage examples in complex APIs (Phase 2 target)
7. ✅ Doctests presence and execution validated
8. ⏳ Error types documentation (Phase 1 target)
9. Cross-references using proper Rust linking
10. LSP workflow integration documentation
11. Enterprise-grade quality standards
12. CI integration and automated quality gates

---

## Summary

### Quick Reference Card (*Diataxis: Reference*)

```bash
# Fastest validation (2-5 min)
just ci-gate

# Full CI validation (10-20 min)
just ci-full

# Local development (fast iteration)
LSP_TEST_FALLBACKS=1 RUST_TEST_THREADS=4 cargo test --workspace --lib

# Debug specific test
RUST_LOG=debug RUST_TEST_THREADS=1 cargo test -p perl-lsp --test failing_test -- --nocapture

# CI configuration
RUST_TEST_THREADS=2 cargo test -p perl-lsp -- --test-threads=2
```

### Key Principles

1. **Adaptive Threading**: Use `RUST_TEST_THREADS=2` for CI, `4+` for local development
2. **Fast Feedback**: Enable `LSP_TEST_FALLBACKS=1` for rapid iteration
3. **Nextest Profiles**: Use `--profile ci` for retry support and slow test detection
4. **Test Count Monitoring**: Enforce 5% drop threshold (720 → 684 tests)
5. **Graceful Degradation**: BrokenPipe errors are expected during teardown, not failures

### Test Execution Time Budget

| Category          | Local Dev | CI (RUST_TEST_THREADS=2) |
|-------------------|-----------|--------------------------|
| Unit tests        | <1s       | <3s                      |
| Integration tests | 5-10s     | 30-60s                   |
| E2E tests         | 10-20s    | 60-120s                  |
| Property tests    | 10-30s    | 60-180s                  |
| **Total**         | **~30s**  | **~5-10 min**            |

---

## Related Documentation

- **[Threading Configuration Guide](THREADING_CONFIGURATION_GUIDE.md)**: Adaptive threading deep dive
- **[Commands Reference](COMMANDS_REFERENCE.md)**: Comprehensive build/test commands
- **[CI Documentation](CI.md)**: CI/CD pipeline architecture
- **[LSP Implementation Guide](LSP_IMPLEMENTATION_GUIDE.md)**: LSP testing patterns
- **[Ignored Tests Index](ci/IGNORED_TESTS_INDEX.md)**: BrokenPipe test tracking
