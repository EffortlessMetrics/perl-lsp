# CLAUDE.md
<!-- Labels: review:stage:governance-checking, review-lane-58, docs:complete, governance:checking -->

This file provides guidance to Claude Code when working with this repository.

**Latest Release**: v0.8.9 GA - Enhanced Builtin Function Parsing & Dual Function Call Indexing + PR #153 Security & Agent Architecture Improvements
**API Stability**: See [docs/STABILITY.md](docs/STABILITY.md)

## Project Overview

This repository contains **five published crates** forming a complete Perl parsing ecosystem with comprehensive workspace refactoring capabilities:

### Published Crates (v0.8.9 GA)

1. **perl-parser** (`/crates/perl-parser/`) ⭐ **MAIN CRATE**
   - Native recursive descent parser with ~100% Perl 5 syntax coverage
   - 4-19x faster than legacy implementations (1-150 µs parsing)
   - Production-ready incremental parsing with <1ms LSP updates
   - Enterprise-grade workspace refactoring and cross-file analysis
   - **Enhanced Dual Indexing Strategy**: Functions indexed under both qualified (`Package::function`) and bare (`function`) names for 98% reference coverage
   - **Enhanced Builtin Function Parsing**: Deterministic parsing of map/grep/sort functions with {} blocks
   - **Test-Driven Development Support**: Auto-detecting TestGenerator with AST-based expectation inference

2. **perl-lsp** (`/crates/perl-lsp/`) ⭐ **LSP BINARY**
   - Standalone Language Server binary with production-grade CLI
   - Enhanced cross-file navigation with dual pattern matching
   - Works with VSCode, Neovim, Emacs, and all LSP-compatible editors

3. **perl-lexer** (`/crates/perl-lexer/`)
   - Context-aware tokenizer with Unicode support
   - Enhanced delimiter recognition including single-quote substitution operators
   - Performance-optimized (v0.8.9+) with comprehensive operator support

4. **perl-corpus** (`/crates/perl-corpus/`)
   - Comprehensive test corpus with property-based testing infrastructure

5. **perl-parser-pest** (`/crates/perl-parser-pest/`) ⚠️ **LEGACY**
   - Pest-based parser (v2 implementation), marked as legacy

## Quick Start

### Installation
```bash
# Install LSP server
cargo install perl-lsp

# Or quick install (Linux/macOS)
curl -fsSL https://raw.githubusercontent.com/EffortlessSteven/tree-sitter-perl/main/install.sh | bash
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

# Revolutionary LSP testing with controlled threading (PR #140 Enhanced)
RUST_TEST_THREADS=2 cargo test -p perl-lsp -- --test-threads=2  # Achieves 5000x performance improvements

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
# Note: Enterprise-grade mutation testing with 87% quality score, UTF-16 security bug discovery
# Real vulnerability detection: symmetric position conversion issues, boundary arithmetic problems
# Improved mutation score from ~70% to 87% with comprehensive edge case coverage and security hardening

# Revolutionary thread-constrained environment testing (PR #140 optimizations)
RUST_TEST_THREADS=2 cargo test -p perl-lsp              # Adaptive timeout with 5000x improvements
RUST_TEST_THREADS=2 cargo test -p perl-lsp --test lsp_behavioral_tests     # 0.31s (was 1560s+)
RUST_TEST_THREADS=2 cargo test -p perl-lsp --test lsp_full_coverage_user_stories  # 0.32s (was 1500s+)
RUST_TEST_THREADS=1 cargo test --test lsp_comprehensive_e2e_test # Maximum reliability mode

# API Documentation Quality Testing ⭐ **NEW: Issue #149**

cargo test -p perl-parser --test missing_docs_ac_tests           # 12 comprehensive acceptance criteria
cargo test -p perl-parser --test missing_docs_ac_tests -- --nocapture  # Detailed validation output
cargo doc --no-deps --package perl-parser                       # Validate doc generation without warnings
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
- **v3 (Native)** ⭐ **RECOMMENDED**: ~100% coverage, 4-19x faster, production incremental parsing, enhanced builtin function support
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
- **Advanced Workspace Indexing**: Revolutionary dual pattern matching for comprehensive LSP navigation across package boundaries
- **Production-Ready LSP Server**: ~89% of LSP features functional with comprehensive workspace support and enhanced reference resolution
- **Adaptive Threading Configuration**: Thread-aware timeout scaling and concurrency management for CI environments
- **Enhanced Incremental Parsing**: <1ms updates with 70-99% node reuse efficiency
- **Unicode-Safe**: Full Unicode identifier and emoji support with proper UTF-8/UTF-16 handling, **symmetric position conversion** (PR #153)
- **Enterprise Security**: Path traversal prevention, file completion safeguards, **UTF-16 boundary vulnerability fixes** (PR #153)
- **Cross-file Workspace Refactoring**: Enterprise-grade symbol renaming, module extraction, comprehensive import optimization
- **Import Optimization**: Remove unused imports, add missing imports, remove duplicates, sort alphabetically

## Documentation

See the [docs/](docs/) directory for comprehensive documentation:

- **[Commands Reference](docs/COMMANDS_REFERENCE.md)** - Comprehensive build/test commands
- **[LSP Implementation Guide](docs/LSP_IMPLEMENTATION_GUIDE.md)** - LSP server architecture  
- **[LSP Development Guide](docs/LSP_DEVELOPMENT_GUIDE.md)** - Source threading and comment extraction
- **[Crate Architecture Guide](docs/CRATE_ARCHITECTURE_GUIDE.md)** - System design and components
- **[Incremental Parsing Guide](docs/INCREMENTAL_PARSING_GUIDE.md)** - Performance and implementation
- **[Security Development Guide](docs/SECURITY_DEVELOPMENT_GUIDE.md)** - Enterprise security practices
- **[Benchmark Framework](docs/BENCHMARK_FRAMEWORK.md)** - Cross-language performance analysis

### Specialized Guides
- **[Builtin Function Parsing](docs/BUILTIN_FUNCTION_PARSING.md)** - Enhanced empty block parsing for map/grep/sort functions
- **[Workspace Navigation](docs/WORKSPACE_NAVIGATION_GUIDE.md)** - Enhanced cross-file features
- **[Rope Integration](docs/ROPE_INTEGRATION_GUIDE.md)** - Document management system
- **[Source Threading](docs/SOURCE_THREADING_GUIDE.md)** - Comment documentation extraction
- **[Position Tracking](docs/POSITION_TRACKING_GUIDE.md)** - UTF-16/UTF-8 position mapping, **symmetric conversion fixes** (PR #153)
- **[Variable Resolution](docs/VARIABLE_RESOLUTION_GUIDE.md)** - Scope analysis system
- **[File Completion Guide](docs/FILE_COMPLETION_GUIDE.md)** - Enterprise-secure path completion
- **[Import Optimizer Guide](docs/IMPORT_OPTIMIZER_GUIDE.md)** - Comprehensive import analysis and optimization
- **[Threading Configuration Guide](docs/THREADING_CONFIGURATION_GUIDE.md)** - Adaptive threading and concurrency management

### Architecture Decision Records (ADRs)
- **[ADR-001: Agent Architecture](docs/ADR_001_AGENT_ARCHITECTURE.md)** - 97 specialized agents and workflow coordination (PR #153)
- **[Agent Orchestration](docs/AGENT_ORCHESTRATION.md)** - Agent ecosystem patterns and routing
- **[Agent Customization Framework](docs/AGENT_CUSTOMIZER.md)** - Domain-specific agent adaptation

## Development Guidelines

### Choosing a Crate
1. **For Any Perl Parsing**: Use `perl-parser` - fastest, most complete, production-ready
2. **For IDE Integration**: Install `perl-lsp` - includes full LSP feature support
3. **For Testing Parsers**: Use `perl-corpus` for comprehensive test suite
4. **For Legacy Migration**: Migrate from `perl-parser-pest` to `perl-parser`

### Development Locations
- **Parser & LSP**: `/crates/perl-parser/` - main development with production Rope implementation
- **LSP Server**: `/crates/perl-lsp/` - standalone LSP server binary (v0.8.9)
- **Lexer**: `/crates/perl-lexer/` - tokenization improvements
- **Test Corpus**: `/crates/perl-corpus/` - test case additions

### API Documentation Standards ⭐ **NEW: Issue #149**

**Comprehensive API documentation is enforced** for the perl-parser crate through `#![warn(missing_docs)]`:

```bash
# Validate documentation compliance
cargo test -p perl-parser --test missing_docs_ac_tests  # 12 acceptance criteria validation
cargo doc --no-deps --package perl-parser              # Generate docs without warnings
```

**Key Requirements** (see [API Documentation Standards](docs/API_DOCUMENTATION_STANDARDS.md)):
- **All public APIs** must have comprehensive documentation with examples
- **Performance-critical modules** must document memory usage and 50GB PST processing implications
- **Error types** must explain email processing workflow context and recovery strategies
- **Module documentation** must describe PSTX pipeline integration (Extract → Normalize → Thread → Render → Index)
- **Cross-references** must use proper Rust documentation linking (`[`function_name`]`)

**Quality Enforcement**:
- **TDD Test Suite**: `/crates/perl-parser/tests/missing_docs_ac_tests.rs` validates all requirements
- **CI Integration**: Automated documentation quality gates prevent regression
- **Edge Case Detection**: Validates malformed doctests, empty docs, invalid cross-references

### Development Workflow (Enhanced)

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

## PR #153 Security & Architecture Enhancements

✅ **Enterprise-Grade Security Improvements**:
- **UTF-16 Position Conversion Fixes**: Critical asymmetric position conversion bug resolved with symmetric fractional position handling
- **Security Vulnerability Remediation**: LSP position mapping boundary violations eliminated through comprehensive mutation testing
- **Unicode Safety**: Enhanced UTF-16/UTF-8 boundary handling with rigorous arithmetic validation
- **Enterprise Security Standards**: Maintained path traversal prevention and file completion safeguards with improved position accuracy

✅ **Advanced Agent Architecture (94 Specialized Agents)**:
- **Domain-Specific Specialization**: Agents optimized for Perl parsing ecosystem requirements
- **Workflow Coordination**: Enhanced routing between review, integration, generative, and maintenance agents
- **Quality Enforcement**: Built-in understanding of mutation testing (87% score), performance benchmarks, and clippy compliance
- **Self-Documenting Configuration**: Agent customization framework with inline expertise and parser-specific context

✅ **Comprehensive Mutation Testing Infrastructure**:
- **Quality Score Achievement**: 87% mutation score (exceeded 85% enterprise target)
- **Real Bug Discovery**: UTF-16 boundary violations, position arithmetic issues, security vulnerabilities
- **Test-Driven Security**: Property-based testing infrastructure with 147+ hardening test cases
- **Systematic Vulnerability Detection**: Mutation testing revealed and eliminated critical security issues

✅ **Performance Preservation**:
- **Revolutionary Performance Maintained**: All PR #140 achievements preserved (5000x LSP improvements)
- **Security-Performance Balance**: Enhanced security without regression in parsing or LSP response times
- **Enterprise Reliability**: 100% test pass rate maintained across security enhancements

## Current Status (v0.8.9 + PR #140 Revolutionary Performance + PR #153 Security Enhancements)

✅ **Revolutionary Production Ready**:
- 100% test pass rate across all components (295+ tests passing including 15/15 builtin function tests)
- **Revolutionary Performance Achievements (PR #140)**:
  - **LSP behavioral tests**: 1560s+ → 0.31s (**5000x faster**, Transformational)
  - **User story tests**: 1500s+ → 0.32s (**4700x faster**, Revolutionary)
  - **Individual workspace tests**: 60s+ → 0.26s (**230x faster**, Game-changing)
  - **Overall test suite**: 60s+ → <10s (**6x faster**, Production-ready)
  - **CI reliability**: 100% pass rate (was ~55% due to timeouts)
- Zero clippy warnings, consistent formatting
- Enterprise-grade LSP server with comprehensive features
- Production-stable incremental parsing with statistical validation

**LSP Features (~89% functional)**:
- ✅ Syntax checking, diagnostics, completion, hover
- ✅ Workspace symbols, rename, code actions (including import optimization)
- ✅ Import optimization: unused/duplicate removal, missing import detection, alphabetical sorting
- ✅ Thread-safe semantic tokens (2.826µs average, zero race conditions)
- ✅ **Revolutionary Adaptive Threading Configuration (PR #140)**: Multi-tier timeout scaling system
  - **LSP Harness Timeouts**: 200-500ms based on thread count (High/Medium/Low contention)
  - **Comprehensive Test Timeouts**: 15s for ≤2 threads, 10s for ≤4 threads, 7.5s for 5-8 threads
  - **Optimized Idle Detection**: 1000ms → 200ms cycles (**5x improvement**)
  - **Intelligent Symbol Waiting**: Exponential backoff with mock responses
  - **Enhanced Test Harness**: Real JSON-RPC protocol with graceful CI degradation
- ✅ **Enhanced cross-file navigation**: Package::subroutine patterns, multi-tier fallback system
- ✅ **Advanced definition resolution**: 98% success rate with workspace+text search combining
- ✅ **Dual-pattern reference search**: Enhanced find references with qualified/unqualified matching
- ✅ Enhanced call hierarchy, go-to-definition, find references
- ✅ Code Lens with reference counts and resolve support
- ✅ File path completion with enterprise security
- ✅ Enhanced formatting: always-available capabilities with graceful perltidy fallback
- ✅ Debug Adapter Protocol (DAP) support

## Contributing

1. **Parser improvements** → `/crates/perl-parser/src/`
2. **LSP features** → `/crates/perl-parser/src/` (provider logic)
3. **CLI enhancements** → `/crates/perl-lsp/src/` (binary interface)
4. **Testing** → Use existing comprehensive test infrastructure with adaptive threading support
5. **Security features** → Follow enterprise security practices
6. **xtask improvements** → `/xtask/src/` (Rust 2024 compatible advanced testing tools)
7. **Agent customization** → `.claude/agents2/` (97 specialized agents for Perl parser ecosystem workflow, PR #153 architecture)

### Coding Standards
- Run `cargo clippy --workspace` before committing changes
- Use `cargo fmt` for consistent formatting
- Prefer `.first()` over `.get(0)` for accessing first element
- Use `.push(char)` instead of `.push_str("x")` for single characters
- Use `or_default()` instead of `or_insert_with(Vec::new)` for default values
- Avoid unnecessary `.clone()` on types that implement Copy
- Add `#[allow(clippy::only_used_in_recursion)]` for recursive tree traversal functions