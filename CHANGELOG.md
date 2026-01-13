# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased] - v0.9.0-semantic-lsp-ready

> **Release checklist** (exit criteria for tagging):
> - [ ] `nix develop -c just ci-gate` green on MSRV
> - [ ] `bash scripts/ignored-test-count.sh` shows BUG=0, MANUAL≤1
> - [ ] README + CURRENT_STATUS + ROADMAP aligned

### Added - Semantic Definition & LSP Integration (Issue #188 Phase 1 Complete - 2025-11-20)

- **Semantic Analyzer Phase 1** - ✅ **COMPLETE (12/12 critical node handlers)**
  - **Critical Node Handlers Implemented**: 12/12 core AST node types for precise semantic analysis
    - Variable declarations: `VariableListDeclaration`, `VariableWithAttributes`
    - Control flow: `Ternary`, `Try`, `PhaseBlock`, `Do`, `Eval`
    - Data structures: `ArrayLiteral`, `HashLiteral`
    - Expressions: `ExpressionStatement`, `Unary`, `Readline`
  - **SemanticModel Stable API**: Production-ready wrapper for semantic analysis
    - `build(root, source)`: Construct semantic model from AST
    - `tokens()`: Access semantic token stream
    - `symbol_table()`: Query symbol definitions
    - `hover_info_at(location)`: Retrieve hover documentation
    - `definition_at(position)`: Resolve symbol definitions by byte offset
  - **Lexical Scoping**: Proper handling of nested scopes, package boundaries, and shadowed variables
  - **Multi-Symbol Support**: Scalars, arrays, hashes, subroutines, and package-qualified calls
  - **Phase 1 Test Coverage**: 13 comprehensive unit tests for semantic analyzer core

- **LSP Semantic Definition Integration** - ✅ **Implementation + Tests Complete**
  - **textDocument/definition Handler**: Integrated with `SemanticAnalyzer::find_definition(byte_offset)`
  - **Precise Symbol Resolution**: Uses semantic analysis instead of heuristic-based text search
  - **Test Infrastructure**: 4 core LSP integration tests with dynamic position calculation
    - Scalar variable definition resolution
    - Subroutine definition resolution
    - Lexical-scoped variable resolution
    - Package-qualified call resolution (`Foo::bar`)
  - **Dynamic Position Calculation**: Tests resilient to whitespace/formatting changes via `find_pos()` helper
  - **Resource-Efficient Testing**: Individual test execution support for constrained environments
  - **Debug Logging**: Comprehensive response logging for troubleshooting (`SCALAR DEF RESPONSE`, etc.)
  - **CI Integration**: `just ci-lsp-def` target for automated validation

- **Statement Tracker & Heredoc Support** - ✅ **100% COMPLETE (Issue #182)**
  - **HeredocContext**: Complete heredoc state tracking with delimiter management
  - **BlockBoundary & BlockType**: Comprehensive block nesting and boundary detection
  - **StatementTracker Integration**: Threaded through entire parser pipeline
  - **HeredocScanner**: Production-ready heredoc content scanning
  - **AST Integration**: Features F1-F6 + edge cases fully validated
  - **Test Coverage**: 274 tests passing at repository level

- **Enhanced Dual Indexing Strategy** - 98% Reference Coverage
  - **Dual Storage Pattern**: Functions indexed under both qualified (`Package::function`) and bare (`function`) names
  - **Dual Retrieval Pattern**: Searches both qualified and bare forms when resolving references
  - **Automatic Deduplication**: Deduplication based on URI + Range to prevent duplicates
  - **Performance Optimized**: Maintains search performance despite dual lookups through efficient indexing
  - **Backward Compatible**: Existing code continues to work with enhanced indexing

- **Enhanced Builtin Function Parsing** (PR #119, Issue #110)
  - **Deterministic Empty Block Parsing**: Map, grep, and sort functions with {} blocks
  - **Block Expression Handling**: Proper parsing of `{ $_ * 2 }` style blocks in builtin functions
  - **Expression vs Statement Disambiguation**: Correctly distinguishes block expressions from hash literals
  - **Edge Case Coverage**: Handles nested blocks, complex expressions, and mixed syntax
  - **Test Coverage**: Comprehensive builtin function parsing tests with edge case validation

- **Enhanced Substitution Operator Parsing** (PR #158)
  - **Complete Pattern/Replacement/Modifier Support**: Full `s///` syntax coverage
  - **All Delimiter Styles**: Balanced delimiters (`s{}{}`, `s[][]`, `s<>`) and alternative delimiters (`s///`, `s###`, `s|||`)
  - **Single-Quote Substitution Delimiters**: Support for `s'pattern'replacement'` syntax
  - **Modifier Support**: All standard Perl modifiers (i, g, s, m, x, e, etc.)
  - **Edge Case Handling**: Complex patterns, escaped delimiters, and nested constructs

- **Enhanced LSP Cancellation System** (PR #165, Issue #48)
  - **Thread-Safe Infrastructure**: `PerlLspCancellationToken` with <100μs check latency and atomic operations
  - **Global Registry**: `CancellationRegistry` for concurrent request coordination and provider cleanup context
  - **JSON-RPC 2.0 Compliance**: Enhanced `$/cancelRequest` handling with LSP 3.17+ features and error response (-32800)
  - **Parser Integration**: Incremental parsing cancellation preserving <1ms updates and workspace navigation capabilities
  - **Performance Optimized**: <50ms end-to-end response time with <1MB memory overhead and thread safety validation
  - **Test Coverage**: 31 test functions across 5 test files covering protocol, performance, parser, infrastructure, and E2E scenarios

- **API Documentation Infrastructure** (PR #160, SPEC-149)
  - **Missing Docs Enforcement**: `#![warn(missing_docs)]` enabled for perl-parser crate
  - **12 Acceptance Criteria Validation**: Comprehensive quality gates and progress tracking
  - **Property-Based Testing**: Fuzz testing with crash/panic detection and AST invariant validation
  - **Mutation Hardening**: 7 mutation hardening test files achieving 60%+ mutation score improvement
  - **Documentation Standards**: Comprehensive API Documentation Standards with LSP workflow integration requirements
  - **CI Integration**: Automated documentation quality gates prevent regression

- **Advanced Parser Robustness** (PR #160, SPEC-149)
  - **Comprehensive Fuzz Testing**: 12 test suites with property-based testing, crash detection, and AST invariant validation
  - **Mutation Testing Enhancement**: 7 mutation hardening test files achieving 60%+ mutation score improvement
  - **Quote Parser Hardening**: Enhanced delimiter handling, boundary validation, and transliteration safety preservation
  - **Production Quality Assurance**: Advanced edge case coverage and real-world scenario testing with systematic vulnerability elimination

- **Test Infrastructure Improvements**
  - **Semantic Unit Tests**: Direct validation of `SemanticAnalyzer` core without LSP overhead
    - `test_analyzer_find_definition_scalar`: Direct analyzer testing
    - `test_semantic_model_definition_at`: SemanticModel API validation
  - **Dynamic Test Positions**: All tests calculate positions from code strings, eliminating brittleness
  - **Resource-Constrained Execution**: Commands optimized for limited CPU/RAM environments
  - **Clear Test Documentation**: Comprehensive command reference in CLAUDE.md and docs

### Added - Debug Adapter Protocol (DAP) Support (Issue #207 - Phase 1)

- **DAP Binary**: New `perl-dap` crate with standalone DAP server
- **Phase 1 Bridge Mode**: Proxies to Perl::LanguageServer for immediate debugging capability
- **Cross-Platform Support**: Windows, macOS, Linux, WSL with automatic path normalization
- **Configuration Management**: Launch (start new process) and attach (connect to running process) modes
- **Enterprise Security**: Path validation, process isolation, and safe defaults
- **Performance**: <50ms breakpoint operations, <100ms step/continue, <200ms variable expansion
- **Quality Assurance**: 71/71 tests passing with comprehensive mutation hardening
- **Documentation**: New `crates/perl-dap/README.md` and enhanced `docs/DAP_USER_GUIDE.md`

### Changed - Project Status & Documentation

- **Core Goal Progress**: Updated to ~80-85% "fully working" for "Perl parser + LSP that actually works"
- **Parser & Heredocs**: Marked as ~95-100% complete - functionally done for v1.0
- **Sprint A**: Marked as ✅ **100% COMPLETE** (parser foundation + heredocs/statement tracker)
- **Sprint B Phase 1**: Semantic analyzer core ✅ complete, ready for Phase 2/3 advanced features
- **MVP Completion**: Updated from 70-75% to 75-80% (parser done, semantics Phase 1 done)
- **Issue #182 (Statement Tracker)**: Ready to close - all implementation and testing complete
- **ROADMAP.md**: Added "Validation & De-Risking" phase with 3-band path to v1.0
- **Status Documentation**: Comprehensive updates to reflect semantic analyzer completion

### Changed - Documentation Truth Alignment (2026-01-07)

- **Roadmap hygiene**: Archived stale `crates/perl-parser/ROADMAP.md` (claimed 35% vs actual 91%)
- **Truth rules**: Added evidence requirements section to canonical roadmap
- **Status table**: Replaced performance claims with evidence-backed stance
- **DAP clarification**: Corrected to "bridge mode" (not "full debugging support")
- **Benchmarks**: Marked as UNVERIFIED until benchmark results are published
- **Milestone planning**: Added concrete v0.9.0 and v1.0.0 exit criteria
- **Local-first workflow**: Emphasized `nix develop -c just ci-gate` as canonical gate

### Performance Improvements (PR #140 - Revolutionary)

- **LSP Behavioral Tests**: 1560s+ → 0.31s (**5000x faster**, Transformational)
- **User Story Tests**: 1500s+ → 0.32s (**4700x faster**, Revolutionary)
- **Individual Workspace Tests**: 60s+ → 0.26s (**230x faster**, Game-changing)
- **Overall Test Suite**: 60s+ → <10s (**6x faster**, Production-ready)
- **CI Reliability**: 100% pass rate (was ~55% due to timeouts)
- **Adaptive Threading Configuration**: Multi-tier timeout scaling system
  - LSP Harness Timeouts: 200-500ms based on thread count (High/Medium/Low contention)
  - Comprehensive Test Timeouts: 15s for ≤2 threads, 10s for ≤4 threads, 7.5s for 5-8 threads
  - Optimized Idle Detection: 1000ms → 200ms cycles (**5x improvement**)
  - Intelligent Symbol Waiting: Exponential backoff with mock responses
  - Enhanced Test Harness: Real JSON-RPC protocol with graceful CI degradation

### Breaking Changes

- **None**: MSRV 1.89, Edition 2024, additive guarantee maintained

### Migration Guide

- **No Migration Required**: v0.8.x users can upgrade seamlessly without code changes
- **Enhanced Features**: All new features are additive and backward compatible
- **API Stability**: Public API contracts remain stable per [docs/STABILITY.md](docs/STABILITY.md)

### Documentation

- **CLAUDE.md**: Added semantic definition testing commands and updated status metrics
- **ROADMAP.md**: New "Current Phase: Validation & De-Risking" section with concrete path forward
- **Path to v1.0**: Documented 3-band approach (prove semantic stack, reduce ignored tests, tag v0.9)
- **Known Constraints**: Resource limits, CI billing, ignored test count (779 tests)
- **Test Commands**: Resource-efficient semantic testing commands for constrained environments
- **perl-dap README.md**: New comprehensive DAP server documentation
- **DAP User Guide**: Enhanced Debug Adapter Protocol setup, configuration, and debugging workflows
- **API Documentation Standards**: Comprehensive documentation enforcement and systematic resolution strategy (PR #160/SPEC-149)
- **LSP Cancellation Architecture Guide**: Complete documentation suite including protocol, architecture, performance, integration, and test strategy guides

## [v0.8.8] - Current Release

See [docs/archive/CHANGELOG.md](docs/archive/CHANGELOG.md) for complete v0.8.8 release notes.

### Highlights

- Enhanced Builtin Function Parsing (PR #119, Issue #110)
- API Documentation Infrastructure (PR #160, SPEC-149)
- Advanced Parser Robustness (fuzz testing, mutation hardening)
- DAP Support Phase 1 (Issue #207)
- Revolutionary LSP Performance (PR #140: 5000x improvements)

## Historical Releases

For complete release history prior to v0.8.8, see:
- [docs/archive/CHANGELOG.md](docs/archive/CHANGELOG.md) - Comprehensive historical changelog
- [docs/archive/CHANGELOG_v0.8.4.md](docs/archive/CHANGELOG_v0.8.4.md) - v0.8.4 release notes
- [docs/archive/CHANGELOG_v3_milestone.md](docs/archive/CHANGELOG_v3_milestone.md) - v3 parser milestone
