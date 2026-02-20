# AGENTS.md
<!-- Labels: governance:validated, parser:comprehensive-improvements, performance:preserved, security:maintained -->

This file provides guidance to Claude Code when working with this repository.

**Latest Release**: v0.9.1 - Initial Public Alpha

**API Stability**: See [docs/STABILITY.md](docs/STABILITY.md)

## Project Overview

This repository contains **six published crates** forming a complete Perl development ecosystem with LSP, DAP, and comprehensive workspace refactoring capabilities:

### Published Crates (0.8.8 + Issue #207 DAP Support)

1. **perl-parser** (`/crates/perl-parser/`) ⭐ **MAIN CRATE**
   - Native recursive descent parser with ~100% Perl 5 syntax coverage
   - Fast native parsing (1-150 µs typical)
   - Incremental parsing with <1ms LSP updates
   - Workspace refactoring and cross-file analysis
   - **Enhanced Dual Indexing Strategy**: Functions indexed under both qualified (`Package::function`) and bare (`function`) names for 98% reference coverage
   - **Enhanced Builtin Function Parsing**: Deterministic parsing of map/grep/sort functions with {} blocks
   - **Test-Driven Development Support**: Auto-detecting TestGenerator with AST-based expectation inference
   - **Comprehensive API Documentation**: Documentation infrastructure with `#![warn(missing_docs)]` enforcement (PR #160/SPEC-149)
   - **Advanced Parser Robustness**: Comprehensive fuzz testing and mutation hardening with 12 test suites (60%+ mutation score improvement)
   - **Documentation Quality Enforcement**: 12 acceptance criteria validation with automated quality gates and progress tracking

2. **perl-lsp** (`/crates/perl-lsp/`) ⭐ **LSP BINARY**
   - Standalone Language Server binary with full-featured CLI
   - Enhanced cross-file navigation with dual pattern matching
   - Works with VSCode, Neovim, Emacs, and all LSP-compatible editors

3. **perl-dap** (`/crates/perl-dap/`) ⭐ **DAP BINARY** (Issue #207 - Phase 1)
   - Debug Adapter Protocol implementation for Perl debugging
   - **Phase 1 Bridge**: Proxies to Perl::LanguageServer for immediate debugging capability
   - Cross-platform support (Windows, macOS, Linux, WSL) with automatic path normalization
   - Security with path validation, process isolation, and safe defaults
   - Performance optimized (<50ms breakpoint operations, <100ms step/continue)
   - Comprehensive testing (71/71 tests passing with mutation hardening)

4. **perl-lexer** (`/crates/perl-lexer/`)
   - Context-aware tokenizer with Unicode support
   - Enhanced delimiter recognition including single-quote substitution operators
   - Performance-optimized (0.8.8+) with comprehensive operator support

5. **perl-corpus** (`/crates/perl-corpus/`)
   - Comprehensive test corpus with property-based testing infrastructure

6. **perl-parser-pest** (`/crates/perl-parser-pest/`) ⚠️ **LEGACY**
   - Pest-based parser (v2 implementation), marked as legacy

## Quick Start

### Installation

```bash
# Install LSP server
cargo install perl-lsp

# Or quick install (Linux/macOS)
curl -fsSL https://raw.githubusercontent.com/EffortlessMetrics/perl-lsp/main/install.sh | bash
```

### Usage

```bash
# Run LSP server (for editors)
perl-lsp --stdio

# Build parser from source
cargo build -p perl-parser --release

# Run tests
cargo test
```

## Essential Commands

**AI tools can run bare `cargo build` and `cargo test`** - the `.cargo/config.toml` ensures correct behavior.

### Build & Install

```bash
# Build main components
cargo build -p perl-lsp --release        # LSP server
cargo build -p perl-parser --release     # Parser library

# Install globally
cargo install perl-lsp                   # From crates.io
cargo install --path crates/perl-lsp     # From source
```

### Testing

```bash
cargo test                               # All tests (robust across environments)
cargo test -p perl-parser               # Parser library tests
cargo test -p perl-lsp                  # LSP server integration tests

# LSP testing with controlled threading (PR #140 Enhanced)
RUST_TEST_THREADS=2 cargo test -p perl-lsp -- --test-threads=2

cargo test -p perl-parser --test lsp_comprehensive_e2e_test -- --nocapture # Full E2E test
cargo test -p perl-parser --test builtin_empty_blocks_test   # Builtin function parsing tests

# Tests pass reliably regardless of external tool availability (perltidy, perlcritic)
# Formatting tests demonstrate graceful degradation when tools are missing

# Test enhanced import optimization features
cargo test -p perl-parser --test import_optimizer_tests   # Import analysis and optimization tests
cargo test -p perl-parser --test import_optimizer_tests -- handles_bare_imports_without_symbols  # Regression-proof bare import analysis

# Test comprehensive substitution operator parsing (PR #158)
cargo test -p perl-parser --test substitution_fixed_tests      # Core substitution operator functionality
cargo test -p perl-parser --test substitution_ac_tests         # Acceptance criteria validation tests
cargo test -p perl-parser --test substitution_debug_test       # Debug verification tests
cargo test -p perl-parser substitution_operator_tests          # Comprehensive substitution syntax coverage

# Test enhanced cross-file navigation capabilities
cargo test -p perl-parser test_cross_file_definition      # Package::subroutine resolution
cargo test -p perl-parser test_cross_file_references      # Enhanced dual-pattern reference search

# Mutation testing and test quality validation (PR #153)
cargo test -p perl-parser --test mutation_hardening_tests # Comprehensive mutation survivor elimination (147 tests)
# Note: Mutation testing with 87% quality score, UTF-16 security bug discovery
# Real vulnerability detection: symmetric position conversion issues, boundary arithmetic problems
# Improved mutation score from ~70% to 87% with comprehensive edge case coverage and security hardening

# Thread-constrained environment testing (PR #140 optimizations)
RUST_TEST_THREADS=2 cargo test -p perl-lsp              # Adaptive timeout
RUST_TEST_THREADS=2 cargo test -p perl-lsp --test lsp_behavioral_tests     # 0.31s
RUST_TEST_THREADS=2 cargo test -p perl-lsp --test lsp_full_coverage_user_stories  # 0.32s
RUST_TEST_THREADS=1 cargo test --test lsp_comprehensive_e2e_test # Maximum reliability mode

# API Documentation Quality Testing ⭐ **NEW: PR #160 (SPEC-149)**
cargo test -p perl-parser --test missing_docs_ac_tests           # 12 comprehensive acceptance criteria
cargo test -p perl-parser --test missing_docs_ac_tests -- --nocapture  # Detailed validation output
cargo doc --no-deps --package perl-parser                       # Validate doc generation without warnings

# Missing Documentation Warnings Infrastructure ⭐ **IMPLEMENTED: PR #160 (SPEC-149)**
# Comprehensive documentation enforcement with `#![warn(missing_docs)]` enabled
# Run: cargo doc --no-deps -p perl-parser 2>&1 | grep -c "missing documentation"  # current count
cargo test -p perl-parser --test missing_docs_ac_tests -- test_missing_docs_warning_compilation  # Verify warnings enabled ✅
cargo test -p perl-parser --test missing_docs_ac_tests -- test_public_functions_documentation_presence  # Function docs (Phase 1 target)
cargo test -p perl-parser --test missing_docs_ac_tests -- test_public_structs_documentation_presence  # Struct/enum docs (Phase 1 target)
cargo test -p perl-parser --test missing_docs_ac_tests -- test_performance_documentation_presence  # Performance docs (Phase 1 target)
cargo test -p perl-parser --test missing_docs_ac_tests -- test_module_level_documentation_presence  # Module docs (Phase 1 target)
cargo test -p perl-parser --test missing_docs_ac_tests -- test_usage_examples_in_complex_apis  # Usage examples (Phase 2 target)
cargo test -p perl-parser --test missing_docs_ac_tests -- test_doctests_presence_and_execution  # Doctests validation ✅
cargo test -p perl-parser --test missing_docs_ac_tests -- test_error_types_documentation  # Error workflow context (Phase 1 target)

# Comprehensive Parser Robustness Testing ⭐ **IMPLEMENTED: PR #160 (SPEC-149)**
# Fuzz Testing Infrastructure - Property-based testing with crash/panic detection
cargo test -p perl-parser --test fuzz_quote_parser_comprehensive  # Bounded fuzz testing with AST invariant validation
cargo test -p perl-parser --test fuzz_quote_parser_simplified     # Focused fuzz testing for regression prevention
cargo test -p perl-parser --test fuzz_quote_parser_regressions    # Known issue reproduction and resolution
cargo test -p perl-parser --test fuzz_incremental_parsing         # Incremental parser stress testing

# Mutation Hardening Tests - Advanced quality assurance with >60% mutation score improvement
cargo test -p perl-parser --test quote_parser_mutation_hardening   # Systematic mutant elimination
cargo test -p perl-parser --test quote_parser_advanced_hardening   # Enhanced edge case coverage
cargo test -p perl-parser --test quote_parser_final_hardening      # Production readiness validation
cargo test -p perl-parser --test quote_parser_realistic_hardening  # Real-world scenario testing

# Semantic Definition Testing ⭐ **NEW: Issue #188 Phase 1 Complete (2025-11-20)**
# Semantic analyzer Phase 1 (12/12 critical node handlers) + LSP textDocument/definition integration
# Tests use dynamic position calculation for robustness across environments

# Semantic Unit Tests (Fast, No LSP) - Direct validation of SemanticAnalyzer core
RUSTC_WRAPPER="" RUST_TEST_THREADS=1 CARGO_BUILD_JOBS=1 \
  cargo test -p perl-parser semantic::tests::test_analyzer_find_definition_scalar -- --nocapture
RUSTC_WRAPPER="" RUST_TEST_THREADS=1 CARGO_BUILD_JOBS=1 \
  cargo test -p perl-parser semantic::tests::test_semantic_model_definition_at -- --nocapture

# LSP Semantic Definition Tests - Resource-efficient (run one at a time on constrained hardware)
RUSTC_WRAPPER="" RUST_TEST_THREADS=1 CARGO_BUILD_JOBS=1 \
  cargo test -p perl-lsp --test semantic_definition -- --nocapture definition_finds_scalar_variable_declaration
RUSTC_WRAPPER="" RUST_TEST_THREADS=1 CARGO_BUILD_JOBS=1 \
  cargo test -p perl-lsp --test semantic_definition -- --nocapture definition_finds_subroutine_declaration
RUSTC_WRAPPER="" RUST_TEST_THREADS=1 CARGO_BUILD_JOBS=1 \
  cargo test -p perl-lsp --test semantic_definition -- --nocapture definition_resolves_scoped_variables
RUSTC_WRAPPER="" RUST_TEST_THREADS=1 CARGO_BUILD_JOBS=1 \
  cargo test -p perl-lsp --test semantic_definition -- --nocapture definition_handles_package_qualified_calls

# CI-ready comprehensive semantic definition validation (requires adequate compute resources)
just ci-lsp-def  # Runs all 4 LSP semantic tests with proper resource constraints
```

### Development

```bash
cargo clippy --workspace                # Lint all crates
cargo bench                             # Run performance benchmarks
perl-lsp --stdio --log                  # Run LSP server with logging
```

### Highlight Testing (*Diataxis: Tutorial* - Tree-Sitter Highlight Test Runner)

```bash
# Prerequisites: Highlight test fixtures in crates/tree-sitter-perl/test/highlight/
# Navigate to xtask directory for highlight testing commands
cd xtask                                 # Navigate to xtask directory

# Run highlight tests with perl-parser AST integration
cargo run highlight                      # Test highlight fixtures with v3 parser
cargo run highlight -- --path ../crates/tree-sitter-perl/test/highlight  # Custom path

# Understanding the output:
# - Total test cases: Number of highlight test fixtures processed
# - Passed/Failed: AST node matching results for expected highlight scopes
# - Integration with perl-corpus: Comprehensive highlight integration tests (4/4 passing)
```

## Architecture

### Crate Structure
- **Core Parser**: `/crates/perl-parser/` - parsing logic, LSP providers, Rope implementation
- **LSP Binary**: `/crates/perl-lsp/` - standalone server, CLI interface, protocol handling
- **Lexer**: `/crates/perl-lexer/` - tokenization, Unicode support
- **Test Corpus**: `/crates/perl-corpus/` - comprehensive test suite
- **Tree-Sitter Integration**: `/crates/tree-sitter-perl-rs/` - unified scanner architecture with Rust delegation pattern
- **xtask**: `/xtask/` - advanced testing tools (excluded from workspace to maintain clean builds)

### Parser Versions
- **v3 (Native)** ⭐ **RECOMMENDED**: ~100% coverage, fast native parsing, incremental parsing, enhanced builtin function support
- **v2 (Pest)**: ~99.996% coverage, legacy but stable
- **v1 (C-based)**: ~95% coverage, benchmarking only (now uses unified Rust scanner via delegation)

### Scanner Architecture (*Diataxis: Explanation* - Unified scanner design)

The scanner implementation uses a unified Rust-based architecture with C compatibility wrapper:

- **Rust Scanner** (`RustScanner`): Core scanning implementation in Rust with full Perl lexical analysis
- **C Scanner Wrapper** (`CScanner`): Compatibility wrapper that delegates to `RustScanner` for legacy API support
- **Unified Implementation**: Both scanner features (`c-scanner` and `rust-scanner`) ultimately use the same Rust code
- **Backward Compatibility**: Existing benchmark and test code continues to work without modification
- **Simplified Maintenance**: Single scanner implementation reduces maintenance overhead while preserving API contracts

## Key Features

- **~100% Perl Syntax Coverage**: Handles all modern Perl constructs including edge cases, enhanced builtin function parsing, **comprehensive substitution operator parsing** (`s///` with complete pattern/replacement/modifier support, all delimiter styles including balanced delimiters `s{}{}, s[][], s<>`, and alternative delimiters `s///, s###, s|||`), and full delimiter support (including single-quote substitution delimiters: `s'pattern'replacement'`)
- **Enhanced Cross-File Navigation**: Dual indexing strategy with 98% reference coverage for both qualified (`Package::function`) and bare (`function`) function calls
- **Advanced Workspace Indexing**: Dual pattern matching for comprehensive LSP navigation across package boundaries
- **LSP Server**: ~91% of LSP features functional with comprehensive workspace support, enhanced reference resolution, and integrated executeCommand capabilities
- **Debug Adapter Protocol (DAP) Support** ⭐ **NEW**: Full debugging support in VS Code and DAP-compatible editors with Phase 1 bridge to Perl::LanguageServer
- **Adaptive Threading Configuration**: Thread-aware timeout scaling and concurrency management for CI environments
- **Enhanced Incremental Parsing**: <1ms updates with 70-99% node reuse efficiency
- **Unicode-Safe**: Full Unicode identifier and emoji support with proper UTF-8/UTF-16 handling, **symmetric position conversion** (PR #153)
- **Security**: Path traversal prevention, file completion safeguards, **UTF-16 boundary vulnerability fixes** (PR #153)
- **Cross-file Workspace Refactoring**: Symbol renaming, module extraction, comprehensive import optimization
- **Import Optimization**: Remove unused imports, add missing imports, remove duplicates, sort alphabetically

## Documentation

See the [docs/](docs/) directory for comprehensive documentation:

- **[Commands Reference](docs/COMMANDS_REFERENCE.md)** - Comprehensive build/test commands
- **[LSP Implementation Guide](docs/LSP_IMPLEMENTATION_GUIDE.md)** - LSP server architecture  
- **[LSP Development Guide](docs/LSP_DEVELOPMENT_GUIDE.md)** - Source threading and comment extraction
- **[Crate Architecture Guide](docs/CRATE_ARCHITECTURE_GUIDE.md)** - System design and components
- **[Incremental Parsing Guide](docs/INCREMENTAL_PARSING_GUIDE.md)** - Performance and implementation
- **[Security Development Guide](docs/SECURITY_DEVELOPMENT_GUIDE.md)** - Security practices
- **[Benchmark Framework](docs/benchmarks/BENCHMARK_FRAMEWORK.md)** - Cross-language performance analysis

### Specialized Guides
- **[DAP User Guide](docs/DAP_USER_GUIDE.md)** ⭐ **NEW** - Debug Adapter Protocol setup, configuration, and debugging workflows
- **[Builtin Function Parsing](docs/BUILTIN_FUNCTION_PARSING.md)** - Enhanced empty block parsing for map/grep/sort functions
- **[Workspace Navigation](docs/WORKSPACE_NAVIGATION_GUIDE.md)** - Enhanced cross-file features
- **[Rope Integration](docs/ROPE_INTEGRATION_GUIDE.md)** - Document management system
- **[Source Threading](docs/SOURCE_THREADING_GUIDE.md)** - Comment documentation extraction
- **[Position Tracking](docs/POSITION_TRACKING_GUIDE.md)** - UTF-16/UTF-8 position mapping, **symmetric conversion fixes** (PR #153)
- **[Variable Resolution](docs/VARIABLE_RESOLUTION_GUIDE.md)** - Scope analysis system
- **[File Completion Guide](docs/FILE_COMPLETION_GUIDE.md)** - Secure path completion
- **[Import Optimizer Guide](docs/IMPORT_OPTIMIZER_GUIDE.md)** - Comprehensive import analysis and optimization
- **[Threading Configuration Guide](docs/THREADING_CONFIGURATION_GUIDE.md)** - Adaptive threading and concurrency management
- **[Error Handling Strategy Guide](docs/ERROR_HANDLING_STRATEGY.md)** - Defensive programming principles and guard condition patterns (Issue #178)
- **[Conditional Documentation Compilation](docs/CONDITIONAL_DOCS_COMPILATION_STRATEGY.md)** - Performance-optimized missing_docs enforcement strategy

### Architecture Decision Records (ADRs)
- **[ADR-001: Agent Architecture](docs/adr/ADR_001_AGENT_ARCHITECTURE.md)** - 97 specialized agents and workflow coordination (PR #153)
- **[ADR-002: API Documentation Infrastructure](docs/ADR_002_API_DOCUMENTATION_INFRASTRUCTURE.md)** - Comprehensive documentation enforcement and systematic resolution strategy (PR #160/SPEC-149)
- **[Agent Orchestration](docs/AGENT_ORCHESTRATION.md)** - Agent ecosystem patterns and routing
- **[Agent Customization Framework](docs/AGENT_CUSTOMIZER.md)** - Domain-specific agent adaptation

## Development Guidelines

### Choosing a Crate
1. **For Any Perl Parsing**: Use `perl-parser` - fast, comprehensive Perl 5 coverage
2. **For IDE Integration**: Install `perl-lsp` - includes full LSP feature support
3. **For Debugging**: Use `perl-dap` - Debug Adapter Protocol support for VS Code and DAP-compatible editors
4. **For Testing Parsers**: Use `perl-corpus` for comprehensive test suite
5. **For Legacy Migration**: Migrate from `perl-parser-pest` to `perl-parser`

### Development Locations
- **Parser & LSP**: `/crates/perl-parser/` - main development with production Rope implementation
- **LSP Server**: `/crates/perl-lsp/` - standalone LSP server binary (0.8.8)
- **DAP Server**: `/crates/perl-dap/` - Debug Adapter Protocol implementation (Issue #207)
- **Lexer**: `/crates/perl-lexer/` - tokenization improvements
- **Test Corpus**: `/crates/perl-corpus/` - test case additions

### API Documentation Standards ⭐ **NEW: PR #160 (SPEC-149)**

**Comprehensive API documentation infrastructure is now enforced** for the perl-parser crate through `#![warn(missing_docs)]`:

```bash
# Validate documentation compliance and infrastructure
cargo test -p perl-parser --test missing_docs_ac_tests  # 12 acceptance criteria validation
cargo doc --no-deps --package perl-parser              # Generate docs without warnings

# Check documentation infrastructure status
cargo test -p perl-parser --test missing_docs_ac_tests -- test_missing_docs_warning_compilation  # Verify enforcement enabled
# Run: cargo doc --no-deps -p perl-parser 2>&1 | grep -c "missing documentation"  # current count
```

**Key Requirements** (see [API Documentation Standards](docs/API_DOCUMENTATION_STANDARDS.md)):
- **All public APIs** must have comprehensive documentation with examples
- **Performance-critical modules** must document parsing performance characteristics and memory usage for large Perl files
- **Error types** must explain Perl parsing context and recovery strategies for syntax errors
- **Module documentation** must describe LSP workflow integration (Parse → Index → Navigate → Complete → Analyze)
- **Cross-references** must use proper Rust documentation linking (`[`function_name`]`)

### Enhanced LSP Cancellation System ⭐ **NEW: PR #165**

**Comprehensive enhanced LSP cancellation infrastructure is now implemented** addressing Issue #48:

```bash
# Validate Enhanced LSP Cancellation System
cargo test -p perl-lsp --test lsp_cancellation_protocol_tests          # Protocol compliance validation
cargo test -p perl-lsp --test lsp_cancellation_performance_tests       # Performance benchmarks validation
cargo test -p perl-lsp --test lsp_cancellation_comprehensive_e2e_tests # End-to-end testing
RUST_TEST_THREADS=2 cargo test -p perl-lsp --test lsp_comprehensive_e2e_test -- --test-threads=2  # Full E2E with threading
```

**Key Features** (see [LSP Cancellation Architecture Guide](docs/CANCELLATION_ARCHITECTURE_GUIDE.md)):
- **Thread-Safe Infrastructure**: `PerlLspCancellationToken` with <100μs check latency and atomic operations
- **Global Registry**: `CancellationRegistry` for concurrent request coordination and provider cleanup context
- **JSON-RPC 2.0 Compliance**: Enhanced `$/cancelRequest` handling with LSP 3.17+ features and error response (-32800)
- **Parser Integration**: Incremental parsing cancellation preserving <1ms updates and workspace navigation capabilities
- **Performance Optimized**: <50ms end-to-end response time with <1MB memory overhead and thread safety validation

**Quality Assurance**:
- **31 Test Functions**: Comprehensive test suite across 5 test files covering protocol, performance, parser, infrastructure, and E2E scenarios
- **16+ Test Fixtures**: Realistic Perl code samples, JSON-RPC protocol fixtures, and performance validation data
- **5 Technical Specifications**: Complete documentation suite including protocol, architecture, performance, integration, and test strategy guides

**Quality Enforcement**:
- **TDD Test Suite**: `/crates/perl-parser/tests/missing_docs_ac_tests.rs` validates all requirements
- **CI Integration**: Automated documentation quality gates prevent regression
- **Edge Case Detection**: Validates malformed doctests, empty docs, invalid cross-references

**Implementation Strategy**:
- **Phased Approach**: See [Documentation Implementation Strategy](docs/DOCUMENTATION_IMPLEMENTATION_STRATEGY.md) for systematic violation resolution
- **Priority-Based Implementation**: Core parser infrastructure → LSP providers → Advanced features → Supporting infrastructure
- **Timeline**: 8-week phased rollout with quality gates and progress tracking

### Development Workflow (Enhanced)

**Local-First Development** - All gates run locally before CI:
```bash
# Canonical local gate (REQUIRED before push)
nix develop -c just ci-gate

# Install pre-push hook (runs gate automatically)
bash scripts/install-githooks.sh

# Gate checks: format, clippy, tests, policy, LSP semantic tests
# Includes nested Cargo.lock detection (prevents subcrate footgun)
```

**Development Server** - Automatic LSP reload on file changes:
```bash
# Start development server with file watching and hot-reload
cd xtask && cargo run --no-default-features -- dev --watch --port 8080

# Features:
# - Monitors Rust (.rs), Perl (.pl, .pm), and config files (.toml)
# - Automatic LSP server restart on changes with 500ms debouncing
# - Graceful shutdown with Ctrl+C
# - Health monitoring and automatic recovery if LSP crashes
# - Cross-platform file watching support
```

**Performance Testing Workflow** - Optimize slow test suites:
```bash
# Analyze test performance and apply optimizations
cd xtask && cargo run --no-default-features -- optimize-tests

# Automatically detects:
# - Long timeout values (>1000ms reduced to 500ms)
# - Excessive wait_for_idle calls (>500ms reduced to 200ms)  
# - Inefficient polling patterns
# - Potential savings up to 3+ seconds per test file
```

**xtask Rust 2024 Compatibility** - Enhanced tooling with modern Rust support:
```bash
# Core functionality (no external dependencies required)
cd xtask && cargo check --no-default-features              # Clean compilation
cd xtask && cargo run --no-default-features -- dev         # Development server
cd xtask && cargo run --no-default-features -- optimize-tests  # Performance optimization

# Advanced features (requires parser-tasks feature and optional libclang)
cd xtask && cargo run --features parser-tasks -- highlight  # Tree-sitter highlight testing

# Features:
# - Rust 2024 edition compatibility with modern `let` expressions
# - Conditional feature gates prevent dependency conflicts
# - Optional tree-sitter integration without breaking core builds
# - Maintains workspace exclusion strategy for clean CI/CD
```
## Dual Indexing Architecture Pattern

When implementing workspace indexing features, follow the dual indexing pattern established in PR #122:

### Implementation Pattern (*Diataxis: Reference* - Code patterns to follow)

```rust
// When indexing function calls, always index under both forms
let qualified = format!("{}::{}", package, bare_name);

// Index under bare name
file_index.references.entry(bare_name.to_string()).or_default().push(symbol_ref.clone());

// Index under qualified name  
file_index.references.entry(qualified).or_default().push(symbol_ref);
```

### Search Pattern (*Diataxis: Reference* - Reference resolution patterns)

```rust
// When searching for references, implement dual pattern matching
pub fn find_references(&self, symbol_name: &str) -> Vec<Location> {
    let mut locations = Vec::new();
    
    // Search exact match first
    if let Some(refs) = index.get(symbol_name) {
        locations.extend(refs.iter().cloned());
    }
    
    // If qualified, also search bare name
    if let Some(idx) = symbol_name.rfind("::") {
        let bare_name = &symbol_name[idx + 2..];
        if let Some(refs) = index.get(bare_name) {
            locations.extend(refs.iter().cloned());
        }
    }
    
    locations
}
```

### Design Principles (*Diataxis: Explanation* - Architectural guidance)

1. **Dual Storage**: Always store function references under both qualified and bare forms
2. **Dual Retrieval**: Always search both qualified and bare forms when resolving references
3. **Automatic Deduplication**: Implement deduplication based on URI + Range to prevent duplicates
4. **Performance Awareness**: Maintain search performance despite dual lookups through efficient indexing
5. **Backward Compatibility**: Ensure existing code continues to work with enhanced indexing

## Current Status (v0.9.1 - Initial Public Alpha)

✅ **Alpha Release** (~85-90% "fully working" for core goal):
- High test pass rate across all components (run `just health` or `cargo test --workspace --lib` for current metrics)
- **Parser & Heredocs/Statement Tracker**: ~95-100% complete - functionally done for v1
- **Semantic Analyzer Phase 1, 2, 3**: ✅ Complete (100% AST node coverage with all handlers implemented)
- **LSP textDocument/definition**: ~90-95% done (implementation + tests complete)
- **Performance Achievements (PR #140)**:
  - **LSP behavioral tests**: 0.31s
  - **User story tests**: 0.32s
  - **Individual workspace tests**: 0.26s
  - **Overall test suite**: <10s
  - **CI reliability**: 100% pass rate
- Zero clippy warnings, consistent formatting
- Comprehensive LSP server with broad feature coverage
- Incremental parsing with statistical validation
- **API Documentation Infrastructure (PR #160/SPEC-149)**:
  - **Successfully Implemented**: `#![warn(missing_docs)]` enforcement with 12 acceptance criteria validation framework
  - **Current Baseline**: Documentation violations tracked for systematic resolution across 4 phases
  - **Quality Assurance**: Property-based testing, edge case detection, and CI integration
  - **Implementation Strategy**: Phased approach targeting critical parser infrastructure first (Phase 1)
  - **Quality Standards**: Comprehensive API Documentation Standards with LSP workflow integration requirements
- **Advanced Parser Robustness (PR #160/SPEC-149)**:
  - **Comprehensive Fuzz Testing**: 12 test suites with property-based testing, crash detection, and AST invariant validation
  - **Mutation Testing Enhancement**: 7 mutation hardening test files achieving 60%+ mutation score improvement
  - **Quote Parser Hardening**: Enhanced delimiter handling, boundary validation, and transliteration safety preservation
  - **Production Quality Assurance**: Advanced edge case coverage and real-world scenario testing with systematic vulnerability elimination

**LSP Features (~92% functional)**:
- ✅ Syntax checking, diagnostics, completion, hover
- ✅ **Semantic-Aware Definition Resolution** ⭐ **COMPLETE: Issue #188 Phase 1, 2, 3**:
  - **SemanticAnalyzer Integration**: `textDocument/definition` uses `find_definition(byte_offset)` for precise symbol resolution
  - **Lexical Scoping**: Proper handling of nested scopes, package boundaries, and shadowed variables
  - **Multi-Symbol Support**: Scalars, arrays, hashes, subroutines, and package-qualified calls
  - **100% AST Node Coverage**: All NodeKind variants have explicit handlers with semantic token generation
  - **Test Coverage**: 33 tests passing (9 new tests for Phase 2/3 handlers)
  - **Dynamic Position Calculation**: Tests resilient to whitespace/formatting changes
  - **Resource-Efficient**: Individual test execution for constrained environments
  - **Performance**: <1ms incremental updates maintained
  - **Code Quality**: Zero clippy warnings across all semantic analyzer code
- ✅ Workspace symbols, rename, advanced code actions (extract variable/subroutine, import optimization, refactoring operations)
- ✅ **Enhanced executeCommand Integration**: Complete LSP executeCommand method support with perl.runCritic command
  - **Dual Analyzer Strategy**: External perlcritic with built-in analyzer fallback for 100% availability
  - **Diagnostic Integration**: Seamless workflow with LSP diagnostic publication pipeline
  - **Performance Optimized**: <50ms code action responses, <2s executeCommand execution
  - **Quality**: Structured error handling with actionable user feedback
- ✅ Import optimization: unused/duplicate removal, missing import detection, alphabetical sorting
- ✅ Thread-safe semantic tokens (2.826µs average, zero race conditions)
- ✅ **Adaptive Threading Configuration (PR #140)**: Multi-tier timeout scaling system
  - **LSP Harness Timeouts**: 200-500ms based on thread count (High/Medium/Low contention)
  - **Comprehensive Test Timeouts**: 15s for ≤2 threads, 10s for ≤4 threads, 7.5s for 5-8 threads
  - **Optimized Idle Detection**: 200ms cycles
  - **Intelligent Symbol Waiting**: Exponential backoff with mock responses
  - **Enhanced Test Harness**: Real JSON-RPC protocol with graceful CI degradation
- ✅ **Enhanced cross-file navigation**: Package::subroutine patterns, multi-tier fallback system
- ✅ **Advanced definition resolution**: 98% success rate with workspace+text search combining
- ✅ **Dual-pattern reference search**: Enhanced find references with qualified/unqualified matching
- ✅ Enhanced call hierarchy, go-to-definition, find references
- ✅ Code Lens with reference counts and resolve support
- ✅ File path completion with security validation
- ✅ Enhanced formatting: always-available capabilities with graceful perltidy fallback
- ✅ **Advanced Code Action Refactorings**: AST-aware refactoring with cross-file impact analysis
  - **Extract Operations**: Variable and subroutine extraction with intelligent parameter detection
  - **Code Quality Improvements**: Convert legacy patterns, add missing pragmas, optimize constructs
  - **Import Management Automation**: Remove unused, add missing, alphabetical sorting with categorization
  - **Workspace-Aware**: Cross-file refactoring with dual indexing safety and 98% reference coverage
- ✅ **Debug Adapter Protocol (DAP) Support** (Issue #207 - Phase 1): Full debugging capabilities in VS Code and DAP-compatible editors
  - **Bridge Architecture**: Proxies to Perl::LanguageServer for immediate debugging availability
  - **Cross-Platform**: Windows, macOS, Linux, WSL with automatic path normalization
  - **Configuration Management**: Launch (start new process) and attach (connect to running process) modes
  - **Performance**: <50ms breakpoint operations, <100ms step/continue, <200ms variable expansion
  - **Security**: Path validation, process isolation, safe defaults
  - **Quality Assurance**: 71/71 tests passing with comprehensive mutation hardening

## GitHub Issues & Project Status

**🎯 Quick Status** (as of 2026-02-19):
- **Core Goal** ("Perl parser + LSP that actually works"): ~85-95% complete
- **MVP**: 85-90% complete (parser done, semantics Phase 1, 2, 3 done, LSP def implementation complete)
- **v0.9.x**: 90-95% ready (validation/de-risking phase)
- **Open Issues**: 30 total (4 ready to close, 2 P0-CRITICAL)
- **Sprint A**: Parser foundation + heredocs/statement tracker ✅ **100% COMPLETE**
- **Sprint B**: LSP polish + semantic analyzer ✅ **100% COMPLETE** (Phase 1, 2, 3 all done)

**📚 Comprehensive Documentation**:
- **[Issue Status Report](docs/ISSUE_STATUS_2025-11-12.md)** - Complete analysis of all 30 open issues with priorities, timelines, and recommendations
- **[Current Status Snapshot](docs/CURRENT_STATUS.md)** - Real-time project health dashboard with metrics and milestones
- **[Production Roadmap (#196)](https://github.com/EffortlessMetrics/perl-lsp/issues/196)** - 6-10 month plan to full production
- **[MVP Roadmap (#195)](https://github.com/EffortlessMetrics/perl-lsp/issues/195)** - 2-3 week plan to minimum viable product

**🚨 Critical Blockers (Immediate Action Required)**:
1. **Issue #211**: CI Pipeline Cleanup - $720/year savings opportunity, 3-week timeline
2. **Issue #210**: Merge-Blocking Gates - Blocked by #211, 8-week implementation
3. ~~**Issue #188**: Semantic Analyzer - Phase 1 ✅ COMPLETE, Phase 2/3 remain for advanced features~~ ✅ **COMPLETE** - All phases (1, 2, 3) done

**✅ Issues Ready to Close**:
- **Issue #182**: Statement Tracker ✅ **100% COMPLETE** (HeredocContext, BlockBoundary, StatementTracker all implemented)
- **Issue #203**: Rust 2024 upgrade ✅ COMPLETE (PR #175)
- **Issue #202**: rand deprecation ✅ COMPLETE (commit e768294f)
- **Issue #194**: Type hierarchy ✅ ALREADY IMPLEMENTED (just needs CI test enablement)

**📋 Sprint Planning**:
- **Sprint A** (Issues #183-186, #182): Parser foundation + heredocs/statement tracker ✅ **100% COMPLETE**
- **Sprint B** (Issues #180, #188, #181, #191): LSP polish + semantic analyzer ✅ **100% COMPLETE** (Phase 1, 2, 3 all done)

## Contributing

1. **Parser improvements** → `/crates/perl-parser/src/`
2. **LSP features** → `/crates/perl-parser/src/` (provider logic)
3. **CLI enhancements** → `/crates/perl-lsp/src/` (binary interface)
4. **DAP features** → `/crates/perl-dap/src/` (Debug Adapter Protocol implementation)
5. **Testing** → Use existing comprehensive test infrastructure with adaptive threading support
6. **Security features** → Follow security practices
7. **xtask improvements** → `/xtask/src/` (Rust 2024 compatible advanced testing tools)
8. **Agent customization** → `.claude/agents2/` (97 specialized agents for Perl parser ecosystem workflow, PR #153 architecture)
9. **Issue tracking** → See [Issue Status Report](docs/ISSUE_STATUS_2025-11-12.md) for priorities and assignments

### Coding Standards

- Run `cargo clippy --workspace` before committing changes
- Use `cargo fmt` for consistent formatting
- Prefer `.first()` over `.get(0)` for accessing first element
- Use `.push(char)` instead of `.push_str("x")` for single characters
- Use `or_default()` instead of `or_insert_with(Vec::new)` for default values
- Avoid unnecessary `.clone()` on types that implement Copy
- Add `#[allow(clippy::only_used_in_recursion)]` for recursive tree traversal functions
- Ban `.unwrap()` in tests and production: do not use `.unwrap()` anywhere. In tests prefer returning `Result` and using the `?` operator, or use `expect()` with a clear, descriptive message only when absolutely necessary.
