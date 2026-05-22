# Commands Reference

This reference lists the commands contributors actually use. The standard flow is:

```bash
just devex
just doctor
just pr-fast
nix develop -c just ci-gate
just ci-full
just status-update
just status-check
just release-check
```

## Tooling Prerequisites

`just` is required for the short command forms used throughout this repository.

```bash
# Install just (https://github.com/casey/just)
cargo install just
```

If you are in a constrained environment where `just` is unavailable, you can still
run the equivalent core checks directly with Cargo:

```bash
# Fast local validation fallback
cargo xtask fmt
cargo test --workspace --lib

# Broader validation fallback
cargo test --workspace
```

## Installation Commands

### LSP Server
```bash
# VS Code extension
code --install-extension EffortlessMetrics.perl-lsp-rs

# GitHub release binary
# Download from https://github.com/EffortlessMetrics/perl-lsp/releases

# Best-effort install script (Linux/macOS)
curl -fsSL https://raw.githubusercontent.com/EffortlessMetrics/perl-lsp/master/install.sh | bash

# Homebrew via the EffortlessMetrics tap (macOS/Linux)
brew install effortlessmetrics/tap/perllsp

# Build from source
cargo build -p perllsp --release

# Install locally from this repo
cargo install --path crates/perllsp

# Run the LSP server
perllsp --stdio  # For editor integration
perllsp --stdio --log  # With debug logging
```

### DAP Server (Debug Adapter)
```bash
# Build DAP server
cargo build -p perl-dap --release

# Install DAP server globally
cargo install --path crates/perl-dap

# Run the DAP server (for VSCode integration)
perl-dap --stdio  # Standard DAP transport
```

## Developer Workflow

### Command Decision Table

| Situation | Command | Why |
|---|---|---|
| New checkout | `just doctor` | Verifies workspace health, hooks, branch state, and common drift. |
| Tool/env check | `just devex` | Checks required tools, Rust components, and local setup. |
| Before push | `just ready` | Runs doctor plus the fast PR gate. |
| Fast PR loop | `just pr-fast` | Cheapest useful proof while iterating. |
| Agent compile/test | `just agent-check` / `just agent-test` | Uses cargo-safe agent profiles and bounded build directories. |
| Agent lint | `just agent-clippy` | Runs clippy through the cargo-safe agent profile. |
| Agent PR proof | `just agent-pr-fast` | Runs the PR-fast gate through cargo-safe. |
| Full pre-merge | `just ci-gate` or `nix develop -c just ci-gate` | Canonical local merge gate. |
| Memory touched | `cargo xtask check-memory-lifecycle-policy` | Enforces retained-state lifecycle and receipt policy. |
| Retained owner added | `cargo xtask check-memory-retained-owner-drift --base origin/master` | Checks whether long-lived storage/task additions need retained-state inventory coverage. |
| Parser-accuracy metrics touched | `just ci-metrics-ratchet-check parser_accuracy` | Verifies parser-accuracy scorecard floors do not regress. |
| Generated status docs touched | `just status-update` then `just status-check` | Regenerates and verifies `docs/project/status/` outputs. |
| Active goal manifest touched | `rtk cargo xtask check-active-goal-manifest` | Verifies the swarm active manifest, lane caps, linked docs, work items, and proof command shape. |
| Release/version surfaces touched | `just version-check` then `just release-check` | Verifies version sync and the release-prep gate before tagging/publishing. |
| Native tooling defaults touched | `cargo xtask native-tooling check-defaults` | Verifies native formatter and native critic default paths do not silently shell out. |
| Native tooling cutover status touched | `cargo xtask native-tooling readiness --markdown docs/project/status/native_tooling_readiness.md` | Renders explicit native formatter/critic default-readiness criteria from existing receipts. |
| Non-Rust migration planning | `cargo xtask non-rust migration-candidates --limit 20` | Finds script-style tooling that should move into Rust-owned crates or typed xtask tasks. |
| User migration check | `perllsp --perltidy-compat-report .perltidyrc` / `perllsp --perlcritic-compat-report .perlcriticrc` | Classifies legacy profiles against native formatter and critic support without requiring external tools. |
| Native critic touched | `cargo xtask native-critic check` | Runs native critic rules and emits check receipts for findings, suppressions, and fixability. |
| DevEx docs touched | `cargo xtask check-devex-docs` | Verifies toolchain wording and documented command references stay current. |
| Need a terminal summary | `just quick-ref` | Prints the short command decision tree. |

### Common Commands

```bash
# Workspace health check (run before any agent-spawning session)
just doctor         # Detects+fixes core.bare, worktree leaks, stale branches, etc.

# Developer utilities
just upstream-log   # Shows recent N commits from auto-detected upstream ref

# Check the local environment (tools, Rust components)
just devex          # Alias: just doctor-env

# Pre-push preflight (doctor + fast gate)
just ready

# Agent-safe compile/test/lint
just agent-preflight
just agent-check
just agent-test
just agent-clippy
just agent-pr-fast

# Fast validation while iterating
just pr-fast

# Canonical local merge gate
nix develop -c just ci-gate

# Metrics/status/release surfaces
just ci-metrics-ratchet-check parser_accuracy
just status-update
just status-check
just version-check
just release-check
```

## Build Commands

### Published Crates and Local Binaries
```bash
# Install the LSP server from this checkout
cargo install --path crates/perllsp        # LSP server

# Add published library crates
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

## Workspace Configuration

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

### Workspace Testing
```bash
# Test all workspace crates
cargo test --workspace --lib            # Library tests only (fast)
cargo test --workspace                  # All tests

# Test individual crates
cargo test -p perl-parser               # Parser tests
cargo test -p perl-lexer                # Lexer tests
cargo test -p perl-lsp-rs                  # LSP server tests
cargo test -p perl-dap                  # DAP server tests
```

### LSP Integration Testing
```bash
# Run with reduced thread count for reliability
RUST_TEST_THREADS=2 cargo test -p perl-lsp-rs -- --test-threads=2

# Run specific test by name
cargo test -p perl-parser -- test_name --exact
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

### LSP Test Threading

The LSP test suite uses adaptive threading. Use `RUST_TEST_THREADS=2` for reliable CI behavior:

```bash
# Reliable CI and development default
RUST_TEST_THREADS=2 cargo test -p perl-lsp-rs -- --test-threads=2

# Override test timeouts for slow environments
LSP_TEST_TIMEOUT_MS=20000 cargo test -p perl-lsp-rs

# Debug failing tests
RUST_LOG=debug LSP_TEST_ECHO_STDERR=1 cargo test -p perl-lsp-rs -- --nocapture
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
# Run LSP tests (recommended thread configuration)
RUST_TEST_THREADS=2 cargo test -p perl-lsp-rs -- --test-threads=2

# Run parser-side LSP unit tests
cargo test -p perl-parser lsp

# Fast mode for CI
RUST_TEST_THREADS=2 cargo test -p perl-lsp-rs -- --test-threads=2

# Run specific performance-sensitive tests with threading control
RUST_TEST_THREADS=2 cargo test -p perl-lsp-rs test_completion_detail_formatting -- --test-threads=2
RUST_TEST_THREADS=2 cargo test -p perl-lsp-rs test_workspace_symbol_search -- --test-threads=2

# Run formatting capability tests (robust across environments)
cargo test -p perl-lsp-rs --test lsp_comprehensive_e2e_test test_e2e_document_formatting
cargo test -p perl-lsp-rs --test lsp_perltidy_test test_formatting_provider_capability

# Test LSP server manually
echo -e 'Content-Length: 58\r\n\r\n{"jsonrpc":"2.0","id":1,"method":"initialize","params":{}}' | perllsp --stdio

# Run with incremental parsing enabled
PERL_LSP_INCREMENTAL=1 perllsp --stdio

# Test incremental parsing with LSP protocol
PERL_LSP_INCREMENTAL=1 perllsp --stdio < test_requests.jsonrpc

# Run with a test file
perllsp --stdio < test_requests.jsonrpc
```

### LSP Testing Environment Variables (*Diataxis: Reference* - Configuration options)

**RUST_TEST_THREADS**:
```bash
# Control test thread concurrency
export RUST_TEST_THREADS=2                # Recommended for CI

# Thread configuration examples:
cargo test -p perl-lsp-rs -- --test-threads=2              # Reliable CI configuration
RUST_TEST_THREADS=1 cargo test -p perl-lsp-rs              # Maximum reliability mode
RUST_TEST_THREADS=4 cargo test -p perl-lsp-rs              # High-performance development
```

**LSP test environment**:
```bash
# Optional external dependencies for compatibility adapters
export PERLTIDY_PATH="/usr/local/bin/perltidy"      # explicit external formatter mode
export PERLCRITIC_PATH="/usr/local/bin/perlcritic"  # explicit legacy critic mode

# Override adaptive test timeouts
LSP_TEST_TIMEOUT_MS=20000 cargo test -p perl-lsp-rs
LSP_TEST_SHORT_MS=1000 cargo test -p perl-lsp-rs

# Debug test output
LSP_TEST_ECHO_STDERR=1 cargo test -p perl-lsp-rs -- --nocapture
```

### LSP executeCommand Integration (*Diataxis: How-to Guide* - Execute command usage)

The LSP server supports `workspace/executeCommand` with native critic analysis,
legacy perlcritic compatibility, and advanced code actions.

#### perl.runCritic Command Usage

**Native Critic and Legacy Compatibility Overview** (*Diataxis: Explanation* - Architecture design):

The normal diagnostic path uses the native critic engine by default. The
`perl.runCritic` execute command still supports legacy compatibility behavior,
but external `perlcritic` should be treated as an explicit adapter rather than
the default editor path.

**Basic Usage** (*Diataxis: Tutorial* - Getting started with code quality analysis):
```bash
# Test perl.runCritic command integration
cargo test -p perl-lsp-rs --test lsp_behavioral_tests -- test_execute_command_perlcritic

# Test executeCommand protocol compliance
cargo test -p perl-lsp-rs --test lsp_execute_command_tests

# Test native critic diagnostics through the LSP runtime
cargo test -p perl-lsp-rs native_critic_engine --profile agent --locked --lib -- --nocapture

# Test execute-command critic behavior in the core provider
cargo test -p perl-lsp-rs test_execute_command_run_critic_builtin --lib

# Test with missing files (error handling)
cargo test -p perl-lsp-rs test_execute_command_run_critic_missing_file --lib
```

**Advanced Configuration** (*Diataxis: How-to Guide* - Optimizing critic integration):

**Native Critic Setup**:
```toml
[diagnostics]
perlcritic = true
perlcritic_severity = 3

[critic]
engine = "native"
profile = "recommended"
```

**External Perlcritic Compatibility Setup**:
```bash
# Install perlcritic only when exact legacy policy output is required
sudo apt-get install perlcritic         # Ubuntu/Debian
brew install perl-critic                # macOS
cpan Perl::Critic                      # CPAN installation

# Verify perlcritic availability
which perlcritic                        # Should return path if installed
perlcritic --version                    # Check version

# Test external analyzer detection
cargo test -p perl-lsp-rs test_command_exists_behavior --profile agent --locked --lib -- --nocapture
```

**Native Critic Capabilities** (*Diataxis: Reference* - Policy coverage):
```rust
// Native critic policies (always available)
- RequireUseStrict: "Missing 'use strict' pragma"
- RequireUseWarnings: "Missing 'use warnings' pragma"
- Syntax::ParseError: "Comprehensive syntax error detection"
- Stable native rule IDs, suppressions, and code actions
- Performance optimized: ~100µs analysis time for typical files
- Parse-error resilient: Continues analysis even with syntax errors
```

**Performance Specifications** (*Diataxis: Reference* - Timing requirements):
| Analyzer Type | File Size | Analysis Time | Policy Coverage | Availability |
|---------------|-----------|---------------|-----------------|--------------|
| Native critic | <10KB | <0.1s | Recommended native profile | 100% availability |
| Native critic | <100KB | <0.3s | Recommended native profile | Parse-error resilient |
| External perlcritic | <10KB | <0.5s | Legacy policy catalog | Explicit compatibility mode |
| External perlcritic | <100KB | <1.5s | Legacy policy catalog | Configurable severity |

**Troubleshooting** (*Diataxis: How-to Guide* - Common issues and solutions):

**Issue: External perlcritic not found in legacy mode**
```bash
# Problem: explicit legacy compatibility mode cannot launch perlcritic
# Solution: use native critic, or install perlcritic and verify PATH
which perlcritic || echo "perlcritic not found in PATH"
echo $PATH | grep -o '/usr/local/bin\|/usr/bin\|/opt/perl/bin'

# Alternative: use the native critic path
cargo test -p perl-lsp-rs test_execute_command_run_critic_builtin --profile agent --locked --lib -- --nocapture
```

**Issue: Analysis timeout or slow performance**
```bash
# Problem: Large files cause timeout
# Solution: Verify file size and complexity
wc -l your_file.pl                     # Check line count
time perlcritic your_file.pl           # Test external tool directly

# Native critic performance validation
cargo test -p perl-lsp-rs test_run_builtin_critic_with_valid_file --profile agent --locked --lib -- --nocapture
```

**Issue: Parse errors prevent analysis**
```bash
# Problem: Syntax errors stop analysis
# Solution: Native critic keeps executeCommand isolated from parser failures
perl -c your_file.pl                   # Check syntax separately
cargo test -p perl-lsp-rs test_execute_command_run_critic_builtin --profile agent --locked --lib -- --nocapture
```

**Integration with LSP Diagnostics** (*Diataxis: How-to Guide* - Diagnostic workflow):
```bash
# Test diagnostic integration with executeCommand
cargo test -p perl-lsp-rs --test lsp_behavioral_tests -- test_execute_command_perlcritic

# Verify diagnostic publication after executeCommand
cargo test -p perl-lsp-rs --test lsp_comprehensive_e2e_test -- test_execute_command_and_code_actions

# Performance validation: <50ms code actions, <2s executeCommand
cargo test -p perl-lsp-rs --test lsp_performance_tests -- test_execute_command_latency
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

**Core Commands**:
```bash
# Test all supported executeCommand operations
cargo test -p perl-lsp-rs --test lsp_execute_command_tests -- test_supported_commands

# Individual command testing
cargo test -p perl-lsp-rs --test lsp_behavioral_tests -- test_execute_command_run_tests     # perl.runTests
cargo test -p perl-lsp-rs --test lsp_behavioral_tests -- test_execute_command_run_file     # perl.runFile
cargo test -p perl-lsp-rs --test lsp_behavioral_tests -- test_execute_command_debug_tests  # perl.debugTests
```

**Command Capabilities**:
- ✅ `perl.runTests` - Execute Perl test files with TAP output parsing
- ✅ `perl.runFile` - Execute single Perl file with output capture
- ✅ `perl.runTestSub` - Execute specific test subroutine with isolation
- ✅ `perl.debugTests` - Debug test execution with breakpoint support
- ✅ `perl.runCritic` - Native critic analysis with explicit Perl::Critic compatibility
- ✅ `perl.explainProviderDecision` - Return a structured provider decision explanation, a user-readable `user_message`, and a local `copyable_payload` for bug reports. The v1 command is conservative: it attaches current provider-matrix receipt anchors for known surfaces, includes the additive `provider_decision.v1` schema version, preserves caller-provided receipt/scenario IDs, accepts an optional object-valued `request_receipt` for request-local bug reports, normalizes attached receipts with shared fallback/source-backed/dynamic-boundary fields while preserving provider-specific fields, redacts workspace roots to class/hash metadata in the copyable payload, can replay persisted provider-local request receipts for covered live rename and refactor proof surfaces, and returns a low-confidence `missing_fact` / `no_result` fallback for unknown surfaces.
- ✅ `perl.workspaceTrustReport` - Return a read-only workspace trust report from current server state, including workspace roots, module-resolution configuration, advisory setup hints, the perldoc oracle contract, sanitized caller-supplied VS Code DAP/perldoc runtime state, launch-configuration/module-path counts and path classes, subprocess probe boundaries, index status, support tiers, provider-decision trace keys, and the report claim boundary. It does not copy raw launch paths, run perldoc, start DAP, inspect debug-session internals, probe Perl, refresh parser receipts, or promote provider support tiers.
- ✅ `perl.explainMissingModuleLookup` - Return a bounded missing-module / `@INC` lookup explanation from current runtime state, including the requested module, expected relative path, effective include paths, PERL5LIB policy, claim boundary, user message, and local copyable payload. It does not scan files, probe Perl, change diagnostic suppression, change resolver behavior, or promote support tiers.
- ✅ `perl.previewSafeDelete` - Return a scoped safe-delete symbol preview with a user-readable allow/block/refuse explanation and an empty workspace edit. This is UX proof only: it never applies live symbol-level deletion.
- ✅ `perl.safeDeleteSymbol` - Return a narrow source-backed symbol-delete `WorkspaceEdit` only when the safe-delete compiler plan is fresh/high-confidence/allowed and rollback proof restores the original text. Imported/exported, stale, generated, dynamic-boundary, low-confidence, fallback, and non-source-backed requests return an empty edit with a blocker/fallback explanation.
- ✅ `perl.previewPackageRename` - Return a scoped package/compiler-backed rename preview with planned edit evidence, blockers or fallback state, and a user-readable explanation. This is UX proof only: it never applies or authorizes package rename edits.

### Advanced Code Actions Testing (*Diataxis: How-to Guide* - Code action workflows)

**Refactoring Operations** (*Diataxis: Tutorial* - Using code actions for refactoring):
```bash
# Test comprehensive code action integration
cargo test -p perl-lsp-rs --test lsp_code_actions_tests

# Test specific refactoring categories
cargo test -p perl-lsp-rs --test lsp_code_actions_tests -- test_extract_variable_action     # RefactorExtract
cargo test -p perl-lsp-rs --test lsp_code_actions_tests -- test_extract_subroutine_action  # Advanced extraction
cargo test -p perl-lsp-rs --test lsp_code_actions_tests -- test_organize_imports_action    # SourceOrganizeImports

# Test code quality improvements
cargo test -p perl-lsp-rs --test lsp_code_actions_tests -- test_modernize_code_actions     # RefactorRewrite
cargo test -p perl-lsp-rs --test lsp_code_actions_tests -- test_add_missing_pragmas_action # Code modernization
```

**Performance Testing** (*Diataxis: How-to Guide* - Code action performance validation):
```bash
# Validate <50ms response time requirement
cargo test -p perl-lsp-rs --test lsp_performance_tests -- test_code_actions_response_time

# Test caching efficiency with incremental updates
cargo test -p perl-lsp-rs --test lsp_code_actions_tests -- test_code_action_caching

# Cross-file refactoring with dual indexing integration
cargo test -p perl-lsp-rs --test lsp_code_actions_tests -- test_cross_file_extract_subroutine
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
cargo test -p perl-lsp-rs --test lsp_comprehensive_e2e_test -- test_execute_command_and_code_actions

# Validate with adaptive threading (recommended)
RUST_TEST_THREADS=2 cargo test -p perl-lsp-rs --test lsp_execute_command_tests -- --test-threads=2
RUST_TEST_THREADS=2 cargo test -p perl-lsp-rs --test lsp_code_actions_tests -- --test-threads=2

# Performance regression prevention
cargo test -p perl-lsp-rs --test lsp_performance_benchmarks -- test_execute_command_latency
cargo test -p perl-lsp-rs --test lsp_performance_benchmarks -- test_code_actions_throughput
```

**Quality Assurance Commands**:
```bash
# executeCommand integration tests
cargo test -p perl-lsp-rs --test lsp_execute_command_tests
cargo test -p perl-lsp-rs --test lsp_code_actions_tests
```

The enhanced executeCommand and code actions integration delivers LSP functionality with <50ms response times, comprehensive error handling, and robust tool integration patterns.

## Benchmark Commands

### Workspace Benchmarks
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

### Comprehensive C vs Rust Benchmark Framework
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

# Custom performance gates
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

### Workspace Quality Checks
```bash
# Run standard Rust quality checks (workspace crates)
cargo fmt                              # Format workspace code
cargo clippy --workspace              # Lint workspace crates  
cargo clippy --workspace --tests      # Lint tests

# Individual crate checks
cargo clippy -p perl-parser           # Lint main parser crate
cargo clippy -p perl-lsp-rs              # Lint LSP server
cargo test --doc                      # Documentation tests

# Legacy quality commands (excluded from workspace)
# cargo xtask check --all             # xtask excluded from workspace
# cargo xtask fmt                     # xtask excluded from workspace
```

### Local Developer Watch Commands
```bash
# Install bacon once for interactive watch mode
cargo install --locked bacon

# Fast watch loop from justfile
just dev-watch            # default: workspace check
just dev-watch-clippy     # core clippy loop
just dev-watch-tests      # core test loop
```

The watch recipes use `bacon.toml` with project-tuned jobs for faster local feedback loops.
For a walkthrough and editor task example, see [Continuous Testing](../how-to/CONTINUOUS_TESTING.md).

## Dual-Scanner Corpus Comparison (*Diataxis: How-to Guide* - Testing procedures)

### Running Dual-Scanner Corpus Tests
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

### Workspace Edge Case Tests
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

## CPAN Corpus Commands (*Diataxis: Reference* - CPAN top-1000 parser validation)

The CPAN corpus workflow validates parser coverage against the top-1000 most-downloaded CPAN distributions. The pipeline has four stages: fetch the distribution list, install the modules locally, sweep (parse) them to measure error rates, and ratchet newly-clean modules into a tracked manifest.

### Justfile Recipes

```bash
# Fetch CPAN top-1000 distribution list from MetaCPAN
just cpan-corpus-fetch              # Writes .ci/cpan-top-1000-distributions.txt

# Install CPAN top-1000 distributions locally via cpanm
just cpan-corpus-install            # Installs into target/cpan-corpus/; auto-fetches the list, bootstraps cpanm, and reuses target/cpan-corpus/.cpanm

# Sweep CPAN corpus and print parser error rates
just cpan-corpus-sweep              # Parse all .pm files, report clean rate

# Seed/update the committed CPAN ratchet floor
just cpan-corpus-baseline-update    # Writes .ci/cpan-corpus-baseline.json

# Check CPAN corpus against baseline and known-clean manifest
just cpan-corpus-check              # Enforces full-corpus ratchet + strict known-clean subset

# Auto-add newly-clean CPAN modules to known-clean manifest
just cpan-corpus-ratchet            # Appends to .ci/cpan-corpus-manifest.txt
```

### xtask Subcommands

```bash
# Fetch distribution list from MetaCPAN
cargo xtask cpan-corpus fetch-list
cargo xtask cpan-corpus fetch-list --output custom-path.txt

# Install distributions locally using cpanm --notest --local-lib
cargo xtask cpan-corpus install
cargo xtask cpan-corpus install --dist-list .ci/cpan-top-1000-distributions.txt --install-dir target/cpan-corpus

# Sweep installed CPAN corpus with the v3 parser
cargo xtask cpan-corpus sweep
cargo xtask cpan-corpus sweep --verbose                    # Per-file details
cargo xtask cpan-corpus sweep --output cpan-report.json    # JSON report

# Auto-ratchet: append newly-clean modules to manifest
cargo xtask cpan-corpus ratchet
cargo xtask cpan-corpus ratchet --manifest .ci/cpan-corpus-manifest.txt
```

### Typical Workflow

```bash
# First-time setup
just cpan-corpus-fetch        # Download distribution list
just cpan-corpus-install      # Install modules (takes a while; fetches the list if needed)
just cpan-corpus-baseline-update  # Commit first ratchet floor

# Ongoing validation (after parser changes)
just cpan-corpus-sweep        # Check current error rates
just cpan-corpus-ratchet      # Lock in improvements

# CI regression check
just cpan-corpus-check        # Fails if full-corpus ratchet or known-clean subset regresses
```

### Prerequisites

- **curl** -- required for `fetch-list` (MetaCPAN API calls)
- **cpanm** -- required for `install` (App::cpanminus)
- **perl** -- required at runtime for module installation

### Key Paths

| What | Where |
|------|-------|
| Distribution list | `.ci/cpan-top-1000-distributions.txt` |
| Full-corpus baseline | `.ci/cpan-corpus-baseline.json` |
| Known-clean manifest | `.ci/cpan-corpus-manifest.txt` |
| Local install directory | `target/cpan-corpus/` |
| Sweep JSON report | `target/cpan-corpus-report.json` (when using `--output`) |

## Release Commands

### Bump Workspace Version

All crate versions inherit from `[workspace.package] version` in `Cargo.toml`. Bump
every tracked version site in a single command:

```bash
just bump-version 0.13.0
```

This updates: `[workspace.package]` version, all `[workspace.dependencies]` version fields,
`vscode-extension/package.json`, `features.toml`, and documentation version references.
Then runs `cargo check --workspace` to regenerate `Cargo.lock`.

After running, review with `git diff`, commit, push, and open a PR.

### Release Sequence

```bash
# 1. Verify all version sites are consistent
just version-check

# 2. Full release gate (ci-gate + release build + version check)
just release-gate

# 3. Extended check (release-gate + semver + changelog + publish dry-run)
just release-check

# 4. After merging the version-bump PR, tag and push
git tag v0.13.0
git push origin v0.13.0
# GitHub Release creation triggers the crates.io publish workflow automatically
```

See [CONTRIBUTING.md](../../CONTRIBUTING.md#release-workflow) for the full release workflow.

## Common Development Tasks

### Adding a New Perl Feature
1. Update `src/grammar.pest` with new syntax rules
2. Add corresponding AST nodes in `pure_rust_parser.rs`
3. Update `build_node()` method to handle new constructs
4. Add tests in `tests/` directory
5. Run tests: `cargo test --features pure-rust`
6. Run benchmarks: `cargo bench --features pure-rust`

### Debugging Parse Failures
1. Use `cargo run -p xtask --features legacy -- corpus --diagnose` for detailed error info
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
