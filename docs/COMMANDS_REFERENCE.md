# Commands Reference (*Diataxis: Reference* - Complete command specifications)

*This reference provides all available commands for building, testing, and using the tree-sitter-perl ecosystem.*

## Installation Commands (*Diataxis: How-to Guide* - Step-by-step installation)

### LSP Server
```bash
# Quick install (Linux/macOS)
curl -fsSL https://raw.githubusercontent.com/EffortlessMetrics/perl-lsp/main/install.sh | bash

# Homebrew (macOS)
brew tap tree-sitter-perl/tap
brew install perl-lsp

# Build from source
cargo build -p perl-lsp --release

# Install globally
cargo install --path crates/perl-lsp

# Run the LSP server
perl-lsp --stdio  # For editor integration
perl-lsp --stdio --log  # With debug logging
```

### DAP Server (Debug Adapter)
```bash
# Build DAP server
cargo build -p perl-parser --bin perl-dap --release

# Install DAP server globally
cargo install --path crates/perl-parser --bin perl-dap

# Run the DAP server (for VSCode integration)
perl-dap --stdio  # Standard DAP transport
```

## Build Commands (*Diataxis: How-to Guide* - Development builds)

### Published Crates
```bash
# Install from crates.io
cargo install perl-lsp                     # LSP server
cargo add perl-parser                      # As library dependency
cargo add perl-corpus --dev                # For testing

# Build from source
cargo build -p perl-parser --release
cargo build -p perl-lexer --release
cargo build -p perl-corpus --release
cargo build -p perl-parser-pest --release  # Legacy
```

### Native Parser (Recommended)
```bash
# Build the lexer and parser
cargo build -p perl-lexer -p perl-parser

# Build with incremental parsing support
cargo build -p perl-parser --features incremental

# Build in release mode
cargo build -p perl-lexer -p perl-parser --release

# Build with incremental parsing in release mode
cargo build -p perl-parser --features incremental --release

# Build everything
cargo build --all
```

## Workspace Configuration (v0.8.8+)

The workspace uses an exclusion strategy to ensure reliable builds across all platforms:

```bash
# Workspace tests (production crates only)
cargo test  # Tests perl-parser, perl-lsp, perl-lexer, perl-corpus

# Check workspace configuration
cargo check  # Should build cleanly without system dependencies

# Workspace status report (see WORKSPACE_TEST_REPORT.md)
# - Excludes tree-sitter-perl-c (requires libclang/system dependencies)
# - Excludes example crates with feature conflicts 
# - Focuses on published crate stability
```

### Workspace Architecture Benefits
- **Clean Builds**: No system dependency failures (libclang, parser.c)
- **CI Stability**: Consistent test results across platforms
- **Production Focus**: Tests only published crate APIs
- **Platform Independence**: Works without tree-sitter C toolchain

### xtask Exclusion Strategy (*Diataxis: Explanation* - Design decisions)
The xtask crate is excluded from the workspace to maintain clean builds while preserving advanced functionality:
- **Why excluded**: xtask depends on excluded crates (tree-sitter-perl-rs with libclang)
- **How to use**: Run from xtask directory: `cd xtask && cargo run <command>`
- **Benefits**: Workspace builds remain system-dependency-free
- **Advanced features**: Dual-scanner corpus comparison requires libclang-dev

## Test Commands

### Workspace Testing (v0.8.8)
```bash
# Test core published crates (workspace members only)
cargo test                              # Tests perl-lexer, perl-parser, perl-corpus, perl-lsp
                                        # Excludes crates with system dependencies

# Test individual published crates
cargo test -p perl-parser               # Main parser library tests (195 tests)
cargo test -p perl-lexer                # Lexer tests (40 tests)  
cargo test -p perl-corpus               # Corpus tests (12 tests)
cargo test -p perl-lsp                  # LSP integration tests

# Advanced test commands (excluded from workspace, run from xtask directory)
# cd xtask && cargo run test            # Advanced xtask test suite
# cd xtask && cargo run corpus          # Dual-scanner corpus comparison
```

### Comprehensive Integration Testing
```bash
# LSP E2E tests
cargo test -p perl-parser --test lsp_comprehensive_e2e_test  # 33 LSP E2E tests

# Symbol documentation tests (comment extraction)
cargo test -p perl-parser --test symbol_documentation_tests

# File completion tests
cargo test -p perl-parser --test file_completion_tests

# DAP tests
cargo test -p perl-parser --test dap_comprehensive_test
cargo test -p perl-parser --test dap_integration_test -- --ignored  # Full integration test

# Incremental parsing tests
cargo test -p perl-parser --test incremental_integration_test --features incremental
cargo test -p perl-parser --features incremental
cargo test -p perl-parser incremental_v2::tests            # IncrementalParserV2 tests

# Performance and benchmark tests  
cargo test -p perl-parser --test incremental_perf_test
cargo bench incremental --features incremental
```

### Enhanced Workspace Navigation Tests (v0.8.8)
```bash
# Test comprehensive AST traversal with ExpressionStatement support
cargo test -p perl-parser --test workspace_comprehensive_traversal_test

# Test enhanced code actions and refactoring
cargo test -p perl-parser code_actions_enhanced

# Test improved call hierarchy provider
cargo test -p perl-parser call_hierarchy_provider

# Test enhanced workspace indexing and symbol resolution
cargo test -p perl-parser workspace_index workspace_rename

# Test TDD basic functionality enhancements
cargo test -p perl-parser tdd_basic
```

### WSL-Safe Local Gate (*Diataxis: How-to Guide* - Resource-constrained testing)

The local gate script provides a reliable test workflow for WSL, containers, and resource-constrained environments by controlling parallelism to prevent OOM crashes.

```bash
# Standard WSL-safe execution (debug build, recommended)
CARGO_BUILD_JOBS=2 RUST_TEST_THREADS=1 ./scripts/gate-local.sh

# Release build mode (faster execution, more memory-intensive)
GATE_RELEASE=1 CARGO_BUILD_JOBS=2 RUST_TEST_THREADS=1 ./scripts/gate-local.sh

# Custom parallelism (for systems with more resources)
CARGO_BUILD_JOBS=4 RUST_TEST_THREADS=2 ./scripts/gate-local.sh
```

**What the gate checks:**
1. **Format check**: `cargo fmt --all -- --check`
2. **Clippy lints**: `cargo clippy --workspace --all-targets -- -D warnings`
3. **Build perl-lsp binary**: Ensures tests use the correct version
4. **Binary version check**: Catches stale/wrong binary issues immediately
5. **perl-parser tests**: Library tests with thread control
6. **perl-lsp tests**: Integration tests with proper binary
7. **perl-lexer tests**: Optional, non-fatal
8. **perl-dap tests**: Optional, non-fatal

**Why this matters:**
- Prevents "mysterious hover null" issues caused by testing against stale binaries
- The `binary_version_test` runs first to catch wrong-binary issues immediately
- Debug binary is built explicitly before tests (avoids stale release binary trap)
- Controlled parallelism prevents WSL OOM crashes
- Works reliably in CI containers with limited resources

**Environment variables:**
| Variable | Default | Description |
|----------|---------|-------------|
| `CARGO_BUILD_JOBS` | 2 | Parallel rustc invocations |
| `RUST_TEST_THREADS` | 1 | Test parallelism (1 = serial) |
| `GATE_RELEASE` | unset | Set to "1" for release builds |

### Revolutionary Performance Testing (PR #140) (*Diataxis: How-to Guide* - Game-changing test reliability)

PR #140 delivers transformative performance optimizations achieving unprecedented test speed and reliability:

- **LSP behavioral tests**: 1560s+ → 0.31s (**5000x faster**)
- **User story tests**: 1500s+ → 0.32s (**4700x faster**)
- **Workspace tests**: 60s+ → 0.26s (**230x faster**)
- **Overall suite**: 60s+ → <10s (**6x faster**)

The testing infrastructure includes sophisticated adaptive threading configuration that scales timeouts and concurrency based on system constraints, enhanced with intelligent symbol waiting and optimized idle detection cycles.

```bash
# Revolutionary CI testing with adaptive timeouts (PR #140 optimizations)
RUST_TEST_THREADS=2 cargo test -p perl-lsp              # 5000x faster behavioral tests
RUST_TEST_THREADS=2 cargo test -p perl-parser           # Enhanced with intelligent symbol waiting

# Optimized single-threaded testing (maximum reliability)
RUST_TEST_THREADS=1 cargo test --test lsp_comprehensive_e2e_test  # Exponential backoff protection

# High-performance development environment
cargo test -p perl-lsp                                   # 200ms idle detection cycles (was 1000ms)
cargo test                                               # <10s total execution (was >60s)

# Enhanced timeout configuration (PR #140 features)
LSP_TEST_TIMEOUT_MS=20000 cargo test -p perl-lsp        # Override adaptive timeouts
LSP_TEST_SHORT_MS=1000 cargo test -p perl-lsp           # Fine-grained timeout control

# Advanced debugging with performance monitoring
LSP_TEST_ECHO_STDERR=1 RUST_TEST_THREADS=2 cargo test -p perl-lsp -- --nocapture
RUST_LOG=debug cargo test -p perl-lsp -- --nocapture    # Monitor timeout scaling
```

#### Revolutionary Performance Matrix (*Diataxis: Reference* - PR #140 achievements)

| Test Suite | Before PR #140 | After PR #140 | Improvement | Strategic Value |
|------------|----------------|----------------|-------------|----------------|
| **lsp_behavioral_tests** | 1560s+ | 0.31s | **5000x faster** | Transformational |
| **lsp_full_coverage_user_stories** | 1500s+ | 0.32s | **4700x faster** | Revolutionary |
| **Individual workspace tests** | 60s+ | 0.26s | **230x faster** | Game-changing |
| **lsp_golden_tests** | 45s | 2.1s | **21x faster** | Significant |
| **Overall test suite** | 60s+ | <10s | **6x faster** | Production-ready |

#### Enhanced Thread Configuration Reference (*Diataxis: Reference* - Multi-tier timeout scaling)

| Environment | Thread Count | LSP Harness | Comprehensive | Idle Detection | Use Case |
|------------|-------------|-------------|--------------|----------------|----------|
| **CI/GitHub Actions** | 0-2 | 500ms | 15s | 200ms cycles | Resource-constrained automation |
| **Constrained Dev** | 3-4 | 300ms | 10s | 200ms cycles | Limited hardware development |
| **Light Constraint** | 5-8 | 200ms | 7.5s | 200ms cycles | Modern development machines |
| **Full Workstation** | >8 | 200ms | 5s | 200ms cycles | High-performance development |

#### Thread-Aware Test Examples (*Diataxis: Tutorial* - Common testing patterns)

```bash
# GitHub Actions CI configuration
- name: Run LSP tests
  run: RUST_TEST_THREADS=2 cargo test -p perl-lsp
  timeout-minutes: 10

# Local development on limited hardware
RUST_TEST_THREADS=4 cargo test -p perl-parser --test lsp_comprehensive_e2e_test

# High-performance workstation (default behavior)
cargo test  # Uses all available threads, standard 5-second timeouts

# Debug timeout issues
RUST_LOG=debug LSP_TEST_ECHO_STDERR=1 RUST_TEST_THREADS=1 cargo test -p perl-lsp --test specific_test
```

## Parser Commands

### Native Parser (perl-parser)
```bash
# Parse a Perl file (create a simple wrapper first)
# The v3 parser is a library - use it programmatically or via examples:

# Test regex patterns including m!pattern!
cargo run -p perl-parser --example test_regex

# Test comprehensive edge cases
cargo run -p perl-parser --example test_edge_cases

# Test all edge cases (shows coverage)
cargo run -p perl-parser --example test_more_edge_cases

# Test LSP capabilities demo
cargo run -p perl-parser --example lsp_capabilities
```

## LSP Development Commands

### Core LSP Testing (*Diataxis: How-to Guide* - Development workflows)

```bash
# Run LSP tests with performance optimizations (v0.8.8+)
cargo test -p perl-parser lsp

# Run LSP integration tests with controlled threading (recommended)
RUST_TEST_THREADS=2 cargo test -p perl-lsp -- --test-threads=2

# Revolutionary performance testing with enhanced test harness (PR #140)
LSP_TEST_FALLBACKS=1 cargo test -p perl-lsp             # Fast mode with mock responses

# Optimal CI performance with adaptive configuration
RUST_TEST_THREADS=2 LSP_TEST_FALLBACKS=1 cargo test -p perl-lsp -- --test-threads=2

# Enhanced test harness features (PR #140)
cargo test -p perl-lsp --test lsp_behavioral_tests       # 5000x performance improvement
cargo test -p perl-lsp --test lsp_full_coverage_user_stories  # 4700x performance improvement

# Run specific performance-sensitive tests with threading control
RUST_TEST_THREADS=2 cargo test -p perl-lsp test_completion_detail_formatting -- --test-threads=2
RUST_TEST_THREADS=2 cargo test -p perl-lsp test_workspace_symbol_search -- --test-threads=2

# Run formatting capability tests (robust across environments)
cargo test -p perl-lsp --test lsp_comprehensive_e2e_test test_e2e_document_formatting
cargo test -p perl-lsp --test lsp_perltidy_test test_formatting_provider_capability

# Test LSP server manually
echo -e 'Content-Length: 58\r\n\r\n{"jsonrpc":"2.0","id":1,"method":"initialize","params":{}}' | perl-lsp --stdio

# Run with incremental parsing enabled (production-ready feature)
PERL_LSP_INCREMENTAL=1 perl-lsp --stdio

# Test incremental parsing with LSP protocol
PERL_LSP_INCREMENTAL=1 perl-lsp --stdio < test_requests.jsonrpc

# Run with a test file
perl-lsp --stdio < test_requests.jsonrpc
```

### LSP Testing Environment Variables (*Diataxis: Reference* - Configuration options)

**RUST_TEST_THREADS** (**Revolutionary Enhancement in PR #140**):
```bash
# Control test thread concurrency for transformative performance
export RUST_TEST_THREADS=2                # Achieves 5000x performance gains in CI

# Revolutionary performance testing with adaptive timeouts
RUST_TEST_THREADS=2 cargo test -p perl-lsp --test lsp_behavioral_tests     # 0.31s (was 1560s+)
RUST_TEST_THREADS=2 cargo test -p perl-lsp --test lsp_full_coverage_user_stories  # 0.32s (was 1500s+)

# Enhanced benefits of PR #140 threading optimization:
# - 5000x faster behavioral test execution
# - 4700x faster user story completion
# - 100% test pass rate (was ~55% due to timeouts)
# - Intelligent symbol waiting with exponential backoff
# - Optimized idle detection (1000ms → 200ms cycles)
# - Enhanced test harness with mock responses and graceful degradation

# Revolutionary thread configuration examples:
cargo test -p perl-lsp -- --test-threads=2              # Optimal CI configuration
RUST_TEST_THREADS=1 cargo test -p perl-lsp              # Maximum reliability mode
RUST_TEST_THREADS=4 cargo test -p perl-lsp              # High-performance development
```

**LSP_TEST_FALLBACKS** (**NEW in v0.8.8**):
```bash
# Enable fast testing mode (reduces test timeouts by ~75%)
export LSP_TEST_FALLBACKS=1

# Optional external dependencies for enhanced features
export PERLTIDY_PATH="/usr/local/bin/perltidy"    # Custom perltidy location
export PERLCRITIC_PATH="/usr/local/bin/perlcritic" # Custom perlcritic location

# Performance characteristics in fallback mode:
# - Base timeout: 500ms (vs 2000ms)
# - Wait for idle: 50ms (vs 2000ms)  
# - Symbol polling: single 200ms attempt (vs progressive backoff)
# - Result: 99.5% faster test execution (60s+ → 0.26s for workspace tests)

# Use cases:
cargo test -p perl-lsp                    # Fast CI/development testing
LSP_TEST_FALLBACKS=1 cargo test --workspace  # Quick workspace validation
LSP_TEST_FALLBACKS=1 cargo check --workspace # Fast build verification
```

**PERL_LSP_INCREMENTAL**:
```bash
# Enable incremental parsing (production-ready)
export PERL_LSP_INCREMENTAL=1
perl-lsp --stdio

# Performance benefits:
# - <1ms LSP updates with 70-99% node reuse efficiency
# - Production-stable incremental parsing
# - Enterprise-grade workspace refactoring support
```

### LSP executeCommand Integration ⭐ **NEW: Issue #145** (*Diataxis: How-to Guide* - Execute command usage)

The LSP server now supports comprehensive `workspace/executeCommand` functionality with integrated perlcritic analysis and advanced code actions.

#### perl.runCritic Command Usage ⭐ **NEW: Issue #145**

**Dual Analyzer Strategy Overview** (*Diataxis: Explanation* - Architecture design):

The `perl.runCritic` command implements a sophisticated dual analyzer strategy ensuring 100% availability:

1. **Primary**: External perlcritic (full policy coverage, configurable)
2. **Fallback**: Built-in analyzer (always available, comprehensive basic policies)
3. **Seamless Transition**: Automatic fallback with no user intervention required
4. **Performance Target**: <2s execution time for typical Perl files

**Basic Usage** (*Diataxis: Tutorial* - Getting started with code quality analysis):
```bash
# Test perl.runCritic command integration
cargo test -p perl-lsp --test lsp_behavioral_tests -- test_execute_command_perlcritic

# Test executeCommand protocol compliance
cargo test -p perl-lsp --test lsp_execute_command_tests

# Test with dual analyzer strategy (external + built-in fallback)
cargo test -p perl-lsp --test lsp_execute_command_tests -- test_perlcritic_dual_analyzer

# Test built-in analyzer specifically
cargo test -p perl-parser --test execute_command_tests -- test_execute_command_run_critic_builtin

# Test with missing files (error handling)
cargo test -p perl-parser --test execute_command_tests -- test_execute_command_run_critic_missing_file
```

**Advanced Configuration** (*Diataxis: How-to Guide* - Optimizing perlcritic integration):

**External Perlcritic Setup**:
```bash
# Install perlcritic for enhanced analysis
sudo apt-get install perlcritic         # Ubuntu/Debian
brew install perl-critic                # macOS
cpan Perl::Critic                      # CPAN installation

# Verify perlcritic availability
which perlcritic                        # Should return path if installed
perlcritic --version                    # Check version

# Test external analyzer detection
cargo test -p perl-parser --test execute_command_tests -- test_command_exists_behavior
```

**Built-in Analyzer Capabilities** (*Diataxis: Reference* - Policy coverage):
```rust
// Built-in analyzer policies (always available)
- RequireUseStrict: "Missing 'use strict' pragma"
- RequireUseWarnings: "Missing 'use warnings' pragma"
- Syntax::ParseError: "Comprehensive syntax error detection"
- Performance optimized: ~100µs analysis time for typical files
- Parse-error resilient: Continues analysis even with syntax errors
```

**Performance Specifications** (*Diataxis: Reference* - Timing requirements):
| Analyzer Type | File Size | Analysis Time | Policy Coverage | Availability |
|---------------|-----------|---------------|-----------------|--------------|
| External perlcritic | <10KB | <0.5s | 150+ policies | Requires installation |
| External perlcritic | <100KB | <1.5s | 150+ policies | Configurable severity |
| Built-in analyzer | <10KB | <0.1s | Core policies | 100% availability |
| Built-in analyzer | <100KB | <0.3s | Core policies | Parse-error resilient |

**Troubleshooting** (*Diataxis: How-to Guide* - Common issues and solutions):

**Issue: External perlcritic not found**
```bash
# Problem: LSP falls back to built-in analyzer always
# Solution: Install perlcritic and verify PATH
which perlcritic || echo "perlcritic not found in PATH"
echo $PATH | grep -o '/usr/local/bin\|/usr/bin\|/opt/perl/bin'

# Alternative: Use built-in analyzer explicitly (always works)
cargo test -p perl-parser --test execute_command_tests -- test_execute_command_run_critic_builtin
```

**Issue: Analysis timeout or slow performance**
```bash
# Problem: Large files cause timeout
# Solution: Verify file size and complexity
wc -l your_file.pl                     # Check line count
time perlcritic your_file.pl           # Test external tool directly

# Built-in analyzer performance validation
cargo test -p perl-parser --test execute_command_tests -- test_run_builtin_critic_with_valid_file
```

**Issue: Parse errors prevent analysis**
```bash
# Problem: Syntax errors stop analysis
# Solution: Built-in analyzer handles parse errors gracefully
perl -c your_file.pl                   # Check syntax separately
cargo test -p perl-parser --test execute_command_tests # Built-in handles syntax errors
```

**Integration with LSP Diagnostics** (*Diataxis: How-to Guide* - Diagnostic workflow):
```bash
# Test diagnostic integration with executeCommand
cargo test -p perl-lsp --test lsp_behavioral_tests -- test_execute_command_perlcritic

# Verify diagnostic publication after executeCommand
cargo test -p perl-lsp --test lsp_comprehensive_e2e_test -- test_execute_command_and_code_actions

# Performance validation: <50ms code actions, <2s executeCommand
cargo test -p perl-lsp --test lsp_performance_tests -- test_execute_command_latency
```

**LSP Protocol Integration** (*Diataxis: Reference* - executeCommand specifications):
```json
// Client request format for perl.runCritic
{
  "jsonrpc": "2.0",
  "id": 1,
  "method": "workspace/executeCommand",
  "params": {
    "command": "perl.runCritic",
    "arguments": ["/path/to/file.pl"]
  }
}

// Server response format
{
  "jsonrpc": "2.0",
  "id": 1,
  "result": {
    "success": true,
    "violations": [
      {
        "policy": "Subroutines::RequireFinalReturn",
        "severity": "medium",
        "message": "Subroutine does not end with explicit return",
        "line": 15,
        "column": 1
      }
    ],
    "analyzer_used": "external",
    "execution_time": "0.125s",
    "file_path": "/path/to/file.pl"
  }
}
```

#### Supported executeCommand Operations (*Diataxis: Reference* - Complete command list)

**Core Commands** (Available since v0.8.8+):
```bash
# Test all supported executeCommand operations
cargo test -p perl-lsp --test lsp_execute_command_tests -- test_supported_commands

# Individual command testing
cargo test -p perl-lsp --test lsp_behavioral_tests -- test_execute_command_run_tests     # perl.runTests
cargo test -p perl-lsp --test lsp_behavioral_tests -- test_execute_command_run_file     # perl.runFile
cargo test -p perl-lsp --test lsp_behavioral_tests -- test_execute_command_debug_tests  # perl.debugTests
```

**Command Capabilities**:
- ✅ `perl.runTests` - Execute Perl test files with TAP output parsing
- ✅ `perl.runFile` - Execute single Perl file with output capture
- ✅ `perl.runTestSub` - Execute specific test subroutine with isolation
- ✅ `perl.debugTests` - Debug test execution with breakpoint support
- ✅ `perl.runCritic` - **NEW**: Perl::Critic analysis with dual analyzer strategy

### Advanced Code Actions Testing ⭐ **NEW: Issue #145** (*Diataxis: How-to Guide* - Code action workflows)

**Refactoring Operations** (*Diataxis: Tutorial* - Using code actions for refactoring):
```bash
# Test comprehensive code action integration
cargo test -p perl-lsp --test lsp_code_actions_tests

# Test specific refactoring categories
cargo test -p perl-lsp --test lsp_code_actions_tests -- test_extract_variable_action     # RefactorExtract
cargo test -p perl-lsp --test lsp_code_actions_tests -- test_extract_subroutine_action  # Advanced extraction
cargo test -p perl-lsp --test lsp_code_actions_tests -- test_organize_imports_action    # SourceOrganizeImports

# Test code quality improvements
cargo test -p perl-lsp --test lsp_code_actions_tests -- test_modernize_code_actions     # RefactorRewrite
cargo test -p perl-lsp --test lsp_code_actions_tests -- test_add_missing_pragmas_action # Code modernization
```

**Performance Testing** (*Diataxis: How-to Guide* - Code action performance validation):
```bash
# Validate <50ms response time requirement
cargo test -p perl-lsp --test lsp_performance_tests -- test_code_actions_response_time

# Test caching efficiency with incremental updates
cargo test -p perl-lsp --test lsp_code_actions_tests -- test_code_action_caching

# Cross-file refactoring with dual indexing integration
cargo test -p perl-lsp --test lsp_code_actions_tests -- test_cross_file_extract_subroutine
```

**LSP Protocol Compliance** (*Diataxis: Reference* - Code action specifications):
```json
// Client request for code actions
{
  "jsonrpc": "2.0",
  "id": 2,
  "method": "textDocument/codeAction",
  "params": {
    "textDocument": {"uri": "file:///path/to/file.pl"},
    "range": {"start": {"line": 10, "character": 4}, "end": {"line": 12, "character": 8}},
    "context": {
      "diagnostics": [],
      "only": ["refactor.extract", "source.organizeImports"]
    }
  }
}

// Server response with available code actions
{
  "jsonrpc": "2.0",
  "id": 2,
  "result": [
    {
      "title": "Extract variable 'user_input'",
      "kind": "refactor.extract",
      "edit": { /* WorkspaceEdit with text changes */ },
      "isPreferred": true
    },
    {
      "title": "Organize Imports",
      "kind": "source.organizeImports",
      "edit": { /* Import optimization changes */ }
    }
  ]
}
```

#### Integration Testing (*Diataxis: How-to Guide* - End-to-end validation)

**Complete Workflow Testing**:
```bash
# Test executeCommand and code actions together
cargo test -p perl-lsp --test lsp_comprehensive_e2e_test -- test_execute_command_and_code_actions

# Validate with adaptive threading (recommended)
RUST_TEST_THREADS=2 cargo test -p perl-lsp --test lsp_execute_command_tests -- --test-threads=2
RUST_TEST_THREADS=2 cargo test -p perl-lsp --test lsp_code_actions_tests -- --test-threads=2

# Performance regression prevention
cargo test -p perl-lsp --test lsp_performance_benchmarks -- test_execute_command_latency
cargo test -p perl-lsp --test lsp_performance_benchmarks -- test_code_actions_throughput
```

**Quality Assurance Commands**:
```bash
# Acceptance criteria validation (Issue #145)
cargo test -p perl-lsp --test lsp_execute_command_tests -- test_ac1_execute_command_implementation  # AC1
cargo test -p perl-lsp --test lsp_execute_command_tests -- test_ac2_perlcritic_integration          # AC2
cargo test -p perl-lsp --test lsp_code_actions_tests -- test_ac3_advanced_refactoring_operations   # AC3

# Previously ignored tests now enabled
cargo test -p perl-lsp --test lsp_behavioral_tests | grep -v "ignored"  # Verify test enablement
```

The enhanced executeCommand and code actions integration delivers enterprise-grade LSP functionality with <50ms response times, comprehensive error handling, and production-ready tool integration patterns.

## Benchmark Commands

### Workspace Benchmarks (v0.8.8)
```bash
# Run parser benchmarks (workspace crates)
cargo bench                             # Benchmarks for published crates
cargo bench -p perl-parser              # Main parser benchmarks (v3)

# Individual crate benchmarks
cargo bench -p perl-lexer               # Lexer performance tests
cargo bench -p perl-corpus              # Corpus validation performance

# Performance validation
cargo test -p perl-parser --test incremental_perf_test  # Incremental parsing performance
```

### Comprehensive C vs Rust Benchmark Framework (v0.8.8)
```bash
# Run complete cross-language benchmark suite with statistical analysis
cargo xtask bench                       # Complete benchmark workflow with C vs Rust comparison

# Individual benchmark components
cargo run -p tree-sitter-perl-rs --bin benchmark_parsers --features pure-rust  # Rust parser benchmarks
cd tree-sitter-perl && node test/benchmark.js  # C implementation benchmarks

# Generate statistical comparison report with configurable thresholds
python3 scripts/generate_comparison.py \
  --c-results c_benchmark.json \
  --rust-results rust_benchmark.json \
  --output comparison.json \
  --report comparison_report.md

# Custom performance gates (5% parse time, 20% memory defaults)
python3 scripts/generate_comparison.py \
  --parse-threshold 3.0 \
  --memory-threshold 15.0 \
  --verbose

# Setup benchmark environment with all dependencies
bash scripts/setup_benchmark.sh

# Run benchmark validation tests (12 comprehensive test cases)
python3 -m pytest scripts/test_comparison.py -v

# Compare all three parsers with memory tracking
cargo xtask compare --report             # Full comparison with memory metrics and statistical analysis
cargo xtask compare --c-only             # Test C implementation only with memory tracking
cargo xtask compare --rust-only          # Test Rust implementation only with memory tracking
cargo xtask compare --validate-only      # Validate existing results without re-running
cargo xtask compare --check-gates        # Check performance gates with memory thresholds

# Memory profiling validation
cargo run --bin xtask -- validate-memory-profiling  # Test dual-mode memory measurement
```

## Code Quality Commands

### Workspace Quality Checks (v0.8.8)
```bash
# Run standard Rust quality checks (workspace crates)
cargo fmt                              # Format workspace code
cargo clippy --workspace              # Lint workspace crates  
cargo clippy --workspace --tests      # Lint tests

# Individual crate checks
cargo clippy -p perl-parser           # Lint main parser crate
cargo clippy -p perl-lsp              # Lint LSP server
cargo test --doc                      # Documentation tests

# Legacy quality commands (excluded from workspace)
# cargo xtask check --all             # xtask excluded from workspace
# cargo xtask fmt                     # xtask excluded from workspace
```

## Dual-Scanner Corpus Comparison (*Diataxis: How-to Guide* - Testing procedures)

### Running Dual-Scanner Corpus Tests (v0.8.8+)
```bash
# Prerequisites: Install libclang-dev for C scanner support
sudo apt-get install libclang-dev  # Ubuntu/Debian
brew install llvm                  # macOS

# Run corpus comparison modes (requires legacy feature)
cargo run -p xtask --features legacy -- corpus                          # Corpus vs selected parser (default scanner: v3)
cargo run -p xtask --features legacy -- corpus --scanner both           # C vs v3 comparison mode
cargo run -p xtask --features legacy -- corpus --scanner both --diagnose

# Individual scanner testing
cargo run -p xtask --features legacy -- corpus --scanner c                    # C scanner
cargo run -p xtask --features legacy -- corpus --scanner rust                 # In-crate v2 pest parser
cargo run -p xtask --features legacy -- corpus --scanner v2-pest-microcrate   # Extracted perl-parser-pest v2
cargo run -p xtask --features legacy -- corpus --scanner v2-parity --diagnose # v2<->v2 drift detector
cargo run -p xtask --features legacy -- corpus --scanner v3                   # V3 parser only

# Diagnostic analysis (*Diataxis: Reference* - detailed comparison)
cargo run -p xtask --features legacy -- corpus --diagnose  # Analyze first failing test
cargo run -p xtask --features legacy -- corpus --test      # Test current parser behavior

# Custom corpus path
cargo run -p xtask --features legacy -- corpus --path tree-sitter-perl/test/corpus
```

### Dual-Scanner Output Analysis (*Diataxis: Explanation* - Understanding results)
```bash
# Scanner mismatch tracking
# When using --scanner both, the system tracks:
# - Total corpus tests run
# - Tests passing both scanners  
# - Tests failing in either scanner
# - Scanner output mismatches (different S-expressions)

# Example output interpretation:
# 📊 Corpus Test Summary:
#    Total: 157
#    Passed: 142 ✅
#    Failed: 15 ❌
#    Scanner mismatches: 23  # C vs Rust differences

# 🔀 Scanner mismatches:
#    corpus_file.txt: test_case_name  # Specific mismatch location
```

### Structural Analysis Features (*Diataxis: Reference* - Analysis capabilities)
```bash
# The dual-scanner system provides:
# - Node count comparison between C and Rust scanners
# - Missing node detection (in C but not Rust output)
# - Extra node detection (in Rust but not C output)  
# - Normalized S-expression comparison (whitespace-independent)
# - Detailed structural diff output for debugging

# Example diagnostic output:
# 🔍 STRUCTURAL ANALYSIS:
# C scanner nodes: 42
# V3 scanner nodes: 41
# ❌ Nodes missing in V3 output:
#   - specific_node_type
# ➕ Extra nodes in V3 output:  
#   - different_node_type
```

### xtask corpus Command Reference (*Diataxis: Reference* - Complete command specification)

```bash
# Basic corpus command structure
cargo run -p xtask --features legacy -- corpus [OPTIONS]

# Command line options:
--path <PATH>              # Corpus directory path (default: tree-sitter-perl/test/corpus)
--scanner <SCANNER>        # Scanner type: c, rust, v2-pest-microcrate, v2-parity, v3, both
--diagnose                 # Run diagnostic analysis on first failing test
--test                     # Test current parser behavior with simple expressions

# Scanner type options:
c       # Use C tree-sitter scanner only (baseline for comparison)
rust    # Use in-crate v2 pest parser (tree_sitter_perl::PureRustPerlParser)
v2-pest-microcrate  # Use extracted perl-parser-pest v2 parser
v2-parity  # Compare in-crate v2 vs extracted v2 output only (ignores corpus expected)
v3      # Use V3 native parser only (perl_parser::Parser)
both    # Compare C scanner vs V3 parser output before corpus expectation check

# Prerequisites for dual-scanner mode:
# Ubuntu/Debian: sudo apt-get install libclang-dev
# macOS: brew install llvm
# Fedora: sudo dnf install clang-devel

# Exit codes:
# 0  - All tests passed, no scanner mismatches
# 1  - Test failures or scanner mismatches detected

# Output format:
# 📊 Corpus Test Summary:
#    Total: <number>         # Total corpus tests processed
#    Passed: <number> ✅     # Tests passing in all scanners
#    Failed: <number> ❌     # Tests failing in any scanner
#    Scanner mismatches: <number>  # Different outputs between scanners
#
# ❌ Failed Tests:           # List of failing tests
#    filename: test_name
#
# 🔀 Scanner mismatches:     # List of scanner differences
#    filename: test_name
```

### Corpus Test File Structure (*Diataxis: Reference* - Test format specification)

```
Test Case Name
================================================================================
source code here
----
(expected s_expression output here)

Next Test Case Name
================================================================================
more source code
----
(expected_output)
```

## Highlight Testing Commands (*Diataxis: Reference* - Tree-Sitter Highlight Test Runner)

### Basic Highlight Testing (*Diataxis: Tutorial* - Getting started with highlight tests)

```bash
# Prerequisites: Navigate to xtask directory for highlight testing
cd xtask

# Run all highlight tests with perl-parser AST integration
cargo run --no-default-features -- highlight

# Test specific highlight directory
cargo run --no-default-features -- highlight --path ../crates/tree-sitter-perl/test/highlight

# Test with specific scanner (for compatibility testing)
cargo run --no-default-features -- highlight --scanner v3
```

### Highlight Integration Testing (*Diataxis: How-to Guide* - Running comprehensive tests)

```bash
# Run perl-corpus highlight integration tests (4 comprehensive tests)
cargo test -p perl-corpus --test highlight_integration_test

# Individual integration test scenarios
cargo test -p perl-corpus highlight_integration_test::test_highlight_runner_integration     # Basic AST integration
cargo test -p perl-corpus highlight_integration_test::test_complex_highlight_constructs    # Complex Perl constructs  
cargo test -p perl-corpus highlight_integration_test::test_highlight_error_handling        # Edge case handling
cargo test -p perl-corpus highlight_integration_test::test_highlight_performance           # Performance validation

# Performance characteristics validation (<100ms for complex code)
cargo test -p perl-corpus highlight_integration_test::test_highlight_performance -- --nocapture
```

### Creating Highlight Test Fixtures (*Diataxis: How-to Guide* - Adding new test cases)

```bash
# Navigate to highlight test fixture directory
cd crates/tree-sitter-perl/test/highlight

# Create new highlight test file (follow existing naming conventions)
touch new_feature.pm

# Highlight test file format:
# Working highlight test examples
# 
# Simple variable assignment
# my $name = "John";
# # <- keyword  
# #    ^ punctuation.special
# #     ^ variable
# #            ^ string
# 
# Number operations  
# 42;
# # <- number
# 
# Use statement
# use strict;
# # <- keyword
# #   ^ type

# Supported highlight scopes mapped to perl-parser AST nodes:
# - keyword        → VariableDeclaration
# - punctuation.special → Variable (sigil mapping)
# - variable       → Variable
# - string         → string
# - number         → number
# - operator       → binary_+ (binary operations)
# - function       → SubDeclaration
# - type           → UseStatement
# - label          → HereDocEnd

# Test your new fixture
cd ../../../../xtask
cargo run --no-default-features -- highlight --path ../crates/tree-sitter-perl/test/highlight
```

### Highlight Test Runner Reference (*Diataxis: Reference* - Complete command specification)

```bash
# Command structure
cd xtask && cargo run --no-default-features -- highlight [OPTIONS]

# Command line options:
--path <PATH>         # Path to highlight test directory [default: c/test/highlight]
--scanner <SCANNER>   # Run with specific scanner [possible values: c, rust, both, v3]

# Default behavior:
# - Uses v3 parser (perl-parser native recursive descent)
# - Processes all .pm files in highlight directory
# - Maps highlight scopes to AST node kinds
# - Reports test results with pass/fail statistics

# Test fixture format requirements:
# - Files must have .pm extension
# - Comments starting with # define expected highlight scopes
# - Source code lines contain the Perl code to be highlighted
# - Empty lines separate test cases within a file
# - Position markers: ^ or <- indicate highlight scope location

# Performance characteristics:
# - ~540ms for 21 test cases (reasonable performance)
# - Integration with comprehensive perl-parser AST traversal
# - Secure path handling with WalkDir max_depth protection
```

### Highlight Test Architecture (*Diataxis: Explanation* - System design and integration)

The highlight test runner integrates deeply with the perl-parser AST generation system:

**Parser Integration**: 
- Uses `perl_parser::Parser` for native recursive descent parsing
- Leverages comprehensive AST node kind collection via `collect_node_kinds()`
- Maps tree-sitter highlight scopes to perl-parser NodeKind variants

**AST Node Mapping Strategy**:
```rust
// Highlight scope → AST NodeKind mapping
"keyword"           → NodeKind::VariableDeclaration
"punctuation.special" → NodeKind::Variable (Perl sigils)
"variable"          → NodeKind::Variable
"string"            → NodeKind::String
"number"            → NodeKind::Number  
"operator"          → NodeKind::Binary with specific operators (+, -, *, etc.)
"function"          → NodeKind::Subroutine
"type"              → NodeKind::Use
```

**Integration with perl-corpus Testing**:
- Comprehensive integration tests validate highlight runner functionality
- 4/4 integration tests passing with performance validation (<100ms)
- Tests cover basic constructs, complex scenarios, error handling, and performance

**Security and Path Handling**:
- Uses `WalkDir` with `max_depth(1)` for secure directory traversal
- Validates file extensions (`.pm` only)
- Proper error handling for parse failures and missing files

**Performance Optimizations**:
- Efficient AST traversal using manual recursion over NodeKind variants
- HashMap-based node counting for fast scope matching
- Progress indication with `indicatif` for user feedback

### Advanced Diagnostic Features (*Diataxis: Reference* - Analysis capabilities)

```bash
# Structural analysis when using --diagnose:
🔍 DIAGNOSTIC: test_name
Input Perl code:
```perl
source code being tested
```

📊 C scanner S-expression:
(program (expression_statement (number "1")))

📊 V3 scanner S-expression:  
(program (expression_statement (literal "1")))

🔍 STRUCTURAL ANALYSIS:
C scanner nodes: 15
V3 scanner nodes: 14
❌ Nodes missing in V3 output:
  - number
➕ Extra nodes in V3 output:
  - literal
```

## Scanner Architecture Testing (*Diataxis: How-to Guide* - Unified scanner validation)

The project uses a unified scanner architecture where both `c-scanner` and `rust-scanner` features use the same Rust implementation, with `CScanner` serving as a compatibility wrapper that delegates to `RustScanner`.

### Scanner Implementation Testing (*Diataxis: Reference* - Scanner validation commands)

```bash
# Test core Rust scanner implementation directly
cargo test -p tree-sitter-perl-rs --features rust-scanner

# Test C scanner wrapper (delegates to Rust implementation internally)
cargo test -p tree-sitter-perl-rs --features c-scanner

# Validate scanner delegation functionality
cargo test -p tree-sitter-perl-rs rust_scanner_smoke

# Test scanner state management and serialization
cargo test -p tree-sitter-perl-rs scanner_state
```

### Scanner Compatibility Validation (*Diataxis: How-to Guide* - Ensuring backward compatibility)

```bash
# Verify both scanner interfaces work correctly
cargo test -p tree-sitter-perl-rs --features rust-scanner,c-scanner

# Test C scanner API compatibility (should delegate to Rust without changes)
cargo test -p tree-sitter-perl-rs c_scanner::tests::test_c_scanner_delegates

# Performance testing (both scanners use same Rust implementation)
cargo bench -p tree-sitter-perl-rs --features rust-scanner
cargo bench -p tree-sitter-perl-rs --features c-scanner
```

### Scanner Build Configuration (*Diataxis: Reference* - Feature flag usage)

```bash
# Build with Rust scanner only (direct usage)
cargo build -p tree-sitter-perl-rs --features rust-scanner

# Build with C scanner wrapper (delegates to Rust internally)
cargo build -p tree-sitter-perl-rs --features c-scanner

# Build with both scanner interfaces available
cargo build -p tree-sitter-perl-rs --features rust-scanner,c-scanner
```

### Understanding Scanner Architecture (*Diataxis: Explanation* - Design rationale)

The unified scanner architecture provides:

- **Single Implementation**: Both `c-scanner` and `rust-scanner` features use the same Rust code
- **Backward Compatibility**: `CScanner` API unchanged, existing benchmark code works without modification  
- **Simplified Maintenance**: One scanner implementation instead of separate C and Rust versions
- **Consistent Performance**: All interfaces benefit from Rust implementation performance

## Edge Case Testing Commands

### Workspace Edge Case Tests (v0.8.8)
```bash  
# Run comprehensive edge case tests (workspace crates)
cargo test -p perl-parser               # Includes all edge case coverage
cargo test -p perl-corpus               # Corpus-based edge case validation

# Specific edge case test suites
cargo test -p perl-parser --test scope_analyzer_tests        # Scope analysis edge cases
cargo test -p perl-parser edge_case                          # Edge case pattern tests
cargo test -p perl-parser regex                              # Regex delimiter tests
cargo test -p perl-parser heredoc                            # Heredoc edge cases
```

## Scope Analyzer Testing

```bash
# Run all scope analyzer tests (38 comprehensive tests)
cargo test -p perl-parser --test scope_analyzer_tests

# Test enhanced variable resolution patterns
cargo test -p perl-parser scope_analyzer_tests::test_hash_access_variable_resolution
cargo test -p perl-parser scope_analyzer_tests::test_array_access_variable_resolution
cargo test -p perl-parser scope_analyzer_tests::test_complex_variable_patterns

# Test hash key context detection
cargo test -p perl-parser scope_analyzer_tests::test_hash_key_context_detection
```

## LSP Development Commands

### Testing Comment Documentation
```bash
# Test comprehensive comment extraction (20 tests covering all scenarios)
cargo test -p perl-parser --test symbol_documentation_tests

# Test specific comment patterns and edge cases
cargo test -p perl-parser symbol_documentation_tests::comment_separated_by_blank_line_is_not_captured
cargo test -p perl-parser symbol_documentation_tests::comment_with_extra_hashes_and_spaces
cargo test -p perl-parser symbol_documentation_tests::multi_package_comment_scenarios
cargo test -p perl-parser symbol_documentation_tests::complex_comment_formatting
cargo test -p perl-parser symbol_documentation_tests::unicode_in_comments
cargo test -p perl-parser symbol_documentation_tests::performance_with_large_comment_blocks

# Performance benchmarking (<100µs per iteration target)
cargo test -p perl-parser symbol_documentation_tests::performance_benchmark_comment_extraction -- --nocapture
```

### Testing Position Tracking
```bash
# Run position tracking tests
cargo test -p perl-parser --test parser_context -- test_multiline_positions
cargo test -p perl-parser --test parser_context -- test_utf16_position_mapping
cargo test -p perl-parser --test parser_context -- test_crlf_line_endings

# Test with specific edge cases
cargo test -p perl-parser parser_context_tests::test_multiline_string_token_positions
```

### Testing File Completion
```bash
# Run file completion specific tests
cargo test -p perl-parser --test file_completion_tests

# Test individual scenarios
cargo test -p perl-parser file_completion_tests::completes_files_in_src_directory
cargo test -p perl-parser file_completion_tests::basic_security_test_rejects_path_traversal

# Test with various file patterns
cargo test -p perl-parser --test lsp_comprehensive_e2e_test -- test_completion
```

## Parser Generation Commands

```bash
# Generate parser from grammar (if needed for testing)
cd tree-sitter-perl
npx tree-sitter generate
```

## Common Development Tasks

### Adding a New Perl Feature
1. Update `src/grammar.pest` with new syntax rules
2. Add corresponding AST nodes in `pure_rust_parser.rs`
3. Update `build_node()` method to handle new constructs
4. Add tests in `tests/` directory
5. Run tests: `cargo test --features pure-rust`
6. Run benchmarks: `cargo bench --features pure-rust`

### Debugging Parse Failures
1. Use `cargo xtask corpus --diagnose` for detailed error info
2. For Pest parser: Check parse error messages which show exact location
3. Use `cargo xtask parse-rust file.pl --ast` to see AST structure

### Performance Optimization
1. Run benchmarks before and after changes: `cargo bench`
2. Use comprehensive benchmark framework: `cargo xtask bench`
3. Use `cargo xtask compare --report` to compare implementations with memory tracking
4. Check performance gates with statistical analysis: `python3 scripts/generate_comparison.py`
5. Check for performance gates: `cargo xtask compare --check-gates`
6. Monitor incremental parsing performance: `cargo test -p perl-parser --test incremental_perf_test`
7. Validate memory profiling: `cargo run --bin xtask -- validate-memory-profiling`
8. Monitor memory usage patterns with statistical analysis
9. Use dual-mode memory measurement (procfs RSS + peak_alloc) for accurate profiling
