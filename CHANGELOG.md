# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

<!-- Next release changes go here -->

## [v0.9.0] - 2026-02-12

### Roadmap (Now / Next / Later)

#### Now (v0.9.1 close-out)

- Index state machine for workspace indexing (state transitions, early-exit, performance caps)
- Documentation cleanup: reduce `missing_docs` violations and complete module-level docs

#### Next (v1.0.0)

- Stability statement (GA-lock + versioning rules)
- Packaging stance (what ships; supported platforms)
- Benchmark publication with receipts under `benchmarks/results/`
- Upgrade notes from v0.8.x → v1.0

#### Later (post v1.0)

- Native DAP completeness (attach, variables/evaluate, safe eval)
- Full LSP 3.18 compliance
- Package manager distribution (Homebrew/apt/etc.)

See `docs/ROADMAP.md` for status receipts and exit criteria.

### Added - Workspace Index Lifecycle v1

- **Explicit build phases**: `Idle → Scanning → Indexing` tracked inside `IndexState::Building`
- **Lifecycle instrumentation**: state/phase durations, transition counts, early-exit reasons
- **Background workspace scan**: initial indexing with performance caps and early-exit reporting

### Added - Notebook + DAP Preview

- **Notebook sync coverage**: added execution summary test coverage and capability gating
- **DAP inline values**: custom `inlineValues` request with regex-based inline value hints and tests

### Changed

- **Index routing**: phase-aware reasons for Building state

### Added - Semantic Analyzer Phase 2-6 Complete (PR #389)

- **Complete NodeKind Coverage**: All remaining AST node handlers implemented
- **Comprehensive Analysis**: Full semantic analysis pipeline for all Perl constructs

### Added - LSP Features & Refactoring

- **Complete LSP Feature Implementation** (PR #387)
  - Window helpers for user interaction
  - Refactoring infrastructure completion

- **Refactoring Engine Enhancements**
  - **Inline Refactoring**: `perform_inline` for variable/expression inlining (PR #391)
  - **Move Code Refactoring**: `perform_move_code` for relocating code blocks (PR #392)

- **TCP Socket Mode** (PR #370): LSP server can now listen on TCP sockets in addition to stdio

- **Uninitialized Variable Detection** (PR #396): Semantic analyzer now detects use of uninitialized variables with improved type inference

- **Non-Interpolated Regexp/Transliteration** (PR #393): Grammar support for non-interpolated content in regexp and transliteration operators

- **Development Server** (PR #395): New xtask development server with live reload capability

- **Syntax Highlighting Test** (PR #397): Actual syntax highlighting validation infrastructure

- **Enhanced S-Expression Formatter**: Improved debugging output with byte-based span types

- **Unified Position/Range Types**: Consolidated position and range types across call hierarchy and inlay hints providers

- **Edge Case Detectors** (PR #373): Manual review edge case detectors for tree-sitter-perl-rs

### Added - VS Code Extension

- **Product Icons** (PR #384): Added icons to commands
- **Run Tests Context Menu** (PR #384): Exposed runTests in editor context menu
- **Inline Variable Command** (PR #335): Improved context menu visibility

### Added - DAP Improvements

- **CLI Argument Parsing** (PR #374): Implemented with clap for better UX
- **Bidirectional Message Proxying**: Enhanced bridge adapter with full duplex communication
- **Async BridgeAdapter** (PR #369): Fully async with graceful shutdown and join!-based proxying
- **Stdio Transport** (PR #330): DAP server stdio transport implementation

### Added - Test Infrastructure

- **Comprehensive Test Corpus** (PR #404): Test corpus infrastructure and DAP documentation updates
- **Workspace Indexing Wait** (PR #394): Robust test assertions with proper indexing synchronization
- **Edge Case and Generator Coverage** (PR #6677faed): Expanded corpus coverage

### Changed - Performance Optimizations

- **O(1) Symbol Lookups** (PR #336): Optimized symbol lookups from linear to constant time
- **ScopeAnalyzer Optimization** (PR #383): Stack-based ancestor tracking for improved performance
- **ScopeAnalyzer Rc Cloning** (PR #621): Eliminated unnecessary Rc cloning in scope analysis for significant performance gains
- **AST Traversal Optimization** (PR #604): Optimized AST traversal in ScopeAnalyzer
- **Parser AST Construction** (PR #367, #372): Reduced string allocations in AST construction
- **Semantic Package Lookup** (PR #368): Return `&str` from `find_current_package` to avoid allocations

### Changed - Architecture

- **Symbol Handling Refactor**: Improved symbol handling in Perl LSP
- **Code Formatting**: Improved code formatting and consistency across multiple files
- **Microcrate Modularization** (PR #601): Production safety improvements with better module organization

### Added - VS Code Extension (2026-01-28)

- **Organize Imports Command** (PR #609): Added to Status Menu for easy access
- **Keybinding Hints** (PR #602): Status Menu now shows keybinding hints
- **Status Menu Improvements**: Separators and settings shortcut for better UX

### Fixed - Security

- **Multi-root Workspace Path Traversal** (PR #620): [HIGH] Fixed path traversal vulnerability in multi-root workspace configurations
- **HTTPS Downgrade Prevention** (PR #603): Prevent HTTPS to HTTP downgrade in binary downloader
- **Safe Evaluation Bypass** (commit 00538b48): Fixed safe evaluation bypass and operator false positives
- **Path Traversal Protection** (PR #388): Complete protection for all execute commands
- **Command Injection Hardening** (PR #332): Hardened executeCommand against injection attacks

### Fixed - Bug Fixes

- **Cross-File Method Resolution** (PR #375): Fixed Package->method call resolution across files
- **Incremental Document Changes** (PR #386): Added error logging for deserialization issues

### Documentation

- **README**: Clarified purpose of legacy parser (PR #8331f9e5)
- **VS Code Config**: Improved configuration setting descriptions (PR #385, #371)
- Clarified perl-dap adapter modes (native CLI vs BridgeAdapter) and current DAP limits (launch-only, placeholder variables/evaluate, attach pending)
- Updated roadmap entries to track DAP adapter wiring and variable/evaluate work

## [v0.9.0] - 2026-01-18

> **Release checklist** (exit criteria for tagging):
> - [x] `nix develop -c just ci-gate` green on MSRV
> - [x] `bash scripts/ignored-test-count.sh` shows BUG=0, MANUAL≤1
> - [x] README + CURRENT_STATUS + ROADMAP aligned

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

### Added - Workspace Refactoring & Advanced LSP Features

- **Advanced Refactoring Capabilities** (Issue #315, #298)
  - **Extract Method Refactoring**: Automated extraction of selected code into new subroutines with parameter detection
  - **Refactoring Rollback**: Transactional safety with `create_backup` infrastructure for atomic operations
  - **Symbol Rename Validation**: Enhanced validation ensuring sigil consistency during rename operations
  - **Refactoring Safety**: Comprehensive validation to prevent invalid code generation

- **LSP 3.18 & Protocol Enhancements**
  - **Inlay Hints**: New resolver for providing parameter names and type hints in the editor
  - **Context-Aware Completions**: Improved completion triggering logic and relevance scoring
  - **Capabilities Resolver**: Modularized capability negotiation for better client compatibility
  - **UTF-16 Wire Types**: Strict compliance with LSP character encoding standards
  - **Execute Command Support** (Issue #145, PR #170):
    - Implemented `workspace/executeCommand` handler
    - Added `perl.runCritic` command for on-demand linting via Perl::Critic
    - **Dual Analyzer Strategy**: Integrated external perlcritic with built-in analyzer fallback

- **Developer Experience & Forensics**
  - **PR Forensics System**: New telemetry infrastructure for tracking CI performance and build metrics
  - **Incremental Metrics**: Assertions to validate incremental parsing efficiency (node reuse rates)
  - **Local-First Guardrails**: Enforced `nix develop` gates to catch issues before CI
  - **Missing Docs Enforcement**: `#![warn(missing_docs)]` now validated in CI (Issue #197)
  - **Documentation Truth System** (Oct 2025):
    - Self-healing documentation system with receipts and renderers
    - Automated verification of documentation against codebase reality

### Added - Core Features & Improvements

- **Statement Tracker & Heredoc Support** - ✅ **100% COMPLETE (Issue #182)**
  - **HeredocContext**: Complete heredoc state tracking with delimiter management
  - **BlockBoundary & BlockType**: Comprehensive block nesting and boundary detection
  - **StatementTracker Integration**: Threaded through entire parser pipeline
  - **HeredocScanner**: Production-ready heredoc content scanning
  - **AST Integration**: Features F1-F6 + edge cases fully validated
  - **Test Coverage**: 274 tests passing at repository level

- **Enhanced Dual Indexing Strategy** - 98% Reference Coverage (Sept 2025)
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
  - **Documentation**: New `docs/BUILTIN_FUNCTION_PARSING.md` guide

- **Enhanced Substitution Operator Parsing** (PR #158, Sept 2025)
  - **Complete Pattern/Replacement/Modifier Support**: Full `s///` syntax coverage
  - **All Delimiter Styles**: Balanced delimiters (`s{}{}`, `s[][]`, `s<>`) and alternative delimiters (`s///`, `s###`, `s|||`)
  - **Single-Quote Substitution Delimiters**: Support for `s'pattern'replacement'` syntax
  - **Modifier Support**: All standard Perl modifiers (i, g, s, m, x, e, etc.)
  - **Edge Case Handling**: Complex patterns, escaped delimiters, and nested constructs

- **Enhanced LSP Cancellation System** (PR #165, Issue #48, Sept 2025)
  - **Thread-Safe Infrastructure**: `PerlLspCancellationToken` with <100μs check latency and atomic operations
  - **Global Registry**: `CancellationRegistry` for concurrent request coordination and provider cleanup context
  - **JSON-RPC 2.0 Compliance**: Enhanced `$/cancelRequest` handling with LSP 3.17+ features and error response (-32800)
  - **Parser Integration**: Incremental parsing cancellation preserving <1ms updates and workspace navigation capabilities
  - **Performance Optimized**: <50ms end-to-end response time with <1MB memory overhead and thread safety validation
  - **Test Coverage**: 31 test functions across 5 test files covering protocol, performance, parser, infrastructure, and E2E scenarios

- **API Documentation Infrastructure** (PR #160, SPEC-149, Sept 2025)
  - **Missing Docs Enforcement**: `#![warn(missing_docs)]` enabled for perl-parser crate
  - **12 Acceptance Criteria Validation**: Comprehensive quality gates and progress tracking
  - **Property-Based Testing**: Fuzz testing with crash/panic detection and AST invariant validation
  - **Mutation Hardening**: 7 mutation hardening test files achieving 60%+ mutation score improvement
  - **Documentation Standards**: Comprehensive API Documentation Standards with LSP workflow integration requirements
  - **CI Integration**: Automated documentation quality gates prevent regression

- **Advanced Parser Robustness** (PR #160, SPEC-149, Oct 2025)
  - **Panic-Safe Architecture**: Elimination of fragile `unreachable!()` macros in parser/lexer (Issue #178)
  - **Comprehensive Fuzz Testing**: 12 test suites with property-based testing, crash detection, and AST invariant validation
  - **Mutation Testing Enhancement**: 7 mutation hardening test files achieving 60%+ mutation score improvement
  - **Quote Parser Hardening**: Enhanced delimiter handling, boundary validation, and transliteration safety preservation
  - **Production Quality Assurance**: Advanced edge case coverage and real-world scenario testing with systematic vulnerability elimination

- **Precision & Syntax Corrections** (Dec 2025 - Jan 2026)
  - **Phase Block Navigation**: Added `name_span` support for precise LSP navigation in `BEGIN`/`CHECK`/`INIT` blocks (Dec 31)
  - **Indirect Object Syntax**: Improved detection of indirect object method calls (e.g., `new ClassName`) (Dec 31)
  - **Moniker QW Parsing**: Tightened parsing rules for `qw(...)` constructs to match perl 5.38 behavior (Dec 31)
  - **Heredoc Edge Cases**: Fixed FIFO body handling and `<<~` indentation edge cases (Nov 2025)

- **Test Infrastructure Improvements**
  - **Semantic Unit Tests**: Direct validation of `SemanticAnalyzer` core without LSP overhead
    - `test_analyzer_find_definition_scalar`: Direct analyzer testing
    - `test_semantic_model_definition_at`: SemanticModel API validation
  - **Dynamic Test Positions**: All tests calculate positions from code strings, eliminating brittleness
  - **Resource-Constrained Execution**: Commands optimized for limited CPU/RAM environments
  - **Cross-Platform Stability**: Tightened `ExitStatus` helpers for consistent behavior on Windows/Linux (Nov 2025)
  - **Clear Test Documentation**: Comprehensive command reference in CLAUDE.md and docs

### Added - Debug Adapter Protocol (DAP) Support (Issue #207 - Phase 1, Oct 2025)

- **DAP Binary**: New `perl-dap` crate with standalone DAP server
- **Phase 1 Bridge Mode**: Proxies to Perl::LanguageServer for immediate debugging capability
- **Cross-Platform Support**: Windows, macOS, Linux, WSL with automatic path normalization
- **Configuration Management**: Launch (start new process) and attach (connect to running process) modes
- **Enterprise Security**: Path validation, process isolation, and safe defaults
- **Performance**: <50ms breakpoint operations, <100ms step/continue, <200ms variable expansion
- **Quality Assurance**: 71/71 tests passing with comprehensive mutation hardening
- **Documentation**: New `crates/perl-dap/README.md` and enhanced `docs/DAP_USER_GUIDE.md`

### Changed - Architecture & Hardening

- **Panic-Safe Architecture** (Issue #143, #292)
  - **Panic Elimination**: Enforced default of `match` or `if let` over `.unwrap()` in production code
  - **Error Propagation**: Comprehensive `Result` types replacing panic-prone operations
  - **Stability Tests**: Verification that server remains stable under malformed input
  - **Macro Safety**: Removed usage of `unreachable!()` in potential code paths in lexer and parser

- **LSP Protocol Modularization** (#297, #309, Dec 2025)
  - **Engine Separation**: Strict separation between core logic (Engine) and protocol handling (LSP)
  - **Handler Modularization**: Individual handlers moved to dedicated modules for maintainability
  - **Fallback/Support Isolation**: Clear boundaries for helper functions and fallback logic

- **Test Infrastructure Stability**
  - **Broken Pipe Elimination**: 123+ tests unignored after fixing IO race conditions (Issue #251)
  - **Test Harness Hardening**: Robust cleanup and process management for test servers
  - **Metric Ratchets**: CI failure on performance regression

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

### Performance Improvements (PR #140 - Revolutionary, Sept 2025)

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

- **Rust 2024 Edition**: Workspace upgraded to Rust 2024 (PR #175, Sept 2025)
- **MSRV Update**: Minimum Supported Rust Version bumped to 1.89+
- **Strict Error Handling**: APIs previously panicking now return `Result` (additive guarantee maintained otherwise)
- **VS Code Extension**: Removed `perl-lsp.downloadBaseUrl` configuration setting (internal archive hosting no longer supported; use standard installation methods)

## [v0.8.9] - 2025-09-09

### Added
- **Cross-File Navigation Enhancements**:
  - Implemented workspace index using `Box<dyn WorkspaceIndex>`
  - Added support for cross-file definitions without workspace index (fallback mechanism) (PR #125)
  - Enhanced cross-referencing capabilities to find references across the entire workspace
- **Import Optimization**:
  - Added new `ImportOptimizer` module to detect and organize imports (PR #96)
  - Implemented sorting and deduplication for `use` statements
  - Added code actions to remove unused imports and add missing imports
- **Builtin Function Parsing (PR #119)**:
  - Enhanced parsing for builtin functions like `map`, `grep`, and `sort`
  - Added support for empty blocks in builtin functions to fix parsing errors
  - Improved handling of block expressions vs hash literals in builtin arguments

### Changed
- **Documentation Updates**:
  - Finalized comprehensive documentation updates with Diataxis framework
  - Added `docs/IMPORT_OPTIMIZER_GUIDE.md`
- **Performance**:
  - Optimized workspace indexing for large codebases

## [v0.8.8] - 2025-09-03

### Added
- **Incremental Parsing V2 (PR #66)**:
  - Enabled incremental node reuse for significantly faster re-parsing on edits
  - Added `IncrementalParserV2` with advanced edit tracking and tree reuse strategies
  - Added benchmarking for incremental parsing performance
- **File Completion**:
  - Implemented enterprise-grade file path completion with security safeguards (PR #58)
  - Added support for relative and absolute paths in `require` and `use` statements
- **Comment Documentation Extraction (PR #71)**:
  - Enhanced extraction of documentation from comments preceding declarations
  - Improved threading of documentation through the parser to the AST
  - Updated Hover provider to display extracted documentation

### Changed
- **LSP Compliance**:
  - Achieved high LSP compliance score with new feature additions
  - Updated architectural documentation to reflect new incremental parsing strategy

## [v0.8.7] - 2025-09-01

### Added
- **Given/When/Default Support (PR #61)**:
  - Added grammar rules and AST nodes for `given`, `when`, and `default` statements (Perl switch statements)
  - Implemented tests for various switch statement scenarios
- **Hash Key Context Detection (PR #68)**:
  - Enhanced Scope Analyzer to detect when barewords are used as hash keys
  - Improved robustness of variable resolution in hash subscript contexts

## [v0.8.6] - 2025-08-30

### Added
- **Enhanced Substitution Parsing (PR #42)**:
  - Support for complex heredoc delimiter expressions
  - Improved regex and substitution parsing with better delimiter handling
  - Fixed regression in S-expression formatting for substitutions
- **Semantic Tokens**:
  - Implemented thread-safe semantic token generation
  - Added verification tests for semantic tokens

## [v0.8.5] - 2025-08-24

### Added
- **LSP Type Hierarchy Support**:
  - Implemented `textDocument/typeDefinition`
  - Added Type Hierarchy provider
- **Pull Diagnostics**:
  - Added support for LSP 3.17 Pull Diagnostics model
- **Symbol Resolution**:
  - Implemented `textDocument/prepareRename` and `textDocument/rename`
  - Added `workspace/symbol` resolution

## [v0.8.4] - 2025-08-23: LSP Feature Complete

### Added
- **LSP 3.17 Features**:
  - Inlay Hints provider
  - Document Links provider
  - Selection Ranges provider
  - On-Type Formatting provider
- **Code Actions**:
  - Implemented robust Code Actions for refactoring and quick fixes
  - Added Rename support

### Changed
- **Architecture**:
  - Modularized LSP capabilities and handlers
  - Enhanced contract-driven testing for LSP features

## [v0.8.3] - 2025-08-22: GA Release

### Added
- **Perl::Critic Integration**:
  - Integrated Perl::Critic for linting and diagnostics
- **Property-Based Testing**:
  - Introduced comprehensive property-based testing framework for parsing robustness
- **Heredoc Improvements**:
  - Robust handling of FIFO bodies and `<<~` indentation
  - Fixed edge cases for heredoc delimiters
- **Release Automation**:
  - Added comprehensive release checklists and scripts

## [v0.8.2] - 2025-08-12

### Added
- **Incremental Parsing Infrastructure**:
  - Foundation work for incremental parsing
  - Environment variable configuration for enabling incremental features
- **Rust 2024 Compatibility**:
  - Updates for Rust 2024 Edition compatibility, specifically `env` operations

## [v0.8.0] - 2025-08-11

### Added
- **Distribution Infrastructure**:
  - VS Code extension marketplace readiness
  - Linux-specific installation steps (ripgrep, shellcheck)
  - Auto-download capability for LSP binary

### Changed
- **Breaking Changes**:
  - Refactored position helpers to be more robust, involving API changes in the parser crate.

## [v0.7.5] - 2025-08-10

### Added
- **Declaration Provider**:
  - Enhanced `DeclarationProvider` with `doc_version` awareness
  - Added assertions for parent map integrity to ensure safe traversal
- **Installer Improvements**:
  - Robust installer with temp dir extraction and checksum support

## [v0.7.4] - 2025-08-08

### Added
- **User Stories**:
  - Added comprehensive user story tests for LSP functionality
  - Achieved 100% user story coverage for key workflows
- **Type Hierarchy**:
  - Added Type Hierarchy provider and related functionality
- **Document Highlight**:
  - Implemented Document Highlight provider

## [v0.7.0] - 2025-08-06

### Added
- **Incremental Parsing (v1)**:
  - Initial support for incremental parsing
- **Folding Ranges**:
  - Implemented Folding Range provider
- **Document Symbols**:
  - Implemented Document Symbol provider
- **Code Lens**:
  - Added Code Lens provider for reference counts and implementations

## [v0.6.0] - 2025-08-04

### Added
- **Debug Adapter Protocol (DAP)**:
  - Initial implementation of Perl Debug Adapter Protocol support
- **Performance**:
  - AST caching and symbol indexing optimizations
- **LSP Features**:
  - Call Hierarchy support
  - Inlay Hints support

## [v0.5.0] - 2025-08-03

### Added
- **Visual Studio Code Extension**:
  - Initial release of the "Perl Language Server" extension
- **Formatting**:
  - Code formatting support integrated with LSP
- **Workspace Symbols**:
  - Implemented Workspace Symbols provider

## [v0.4.0] - 2025-07-25

### Added
- **v3 Parser Completion**:
  - Achieved 100% edge case coverage for Perl 5 syntax
- **Error Recovery**:
  - Implemented robust error recovery strategies in the parser
- **Streaming Parsing**:
  - Added support for streaming parsing input

## [v0.3.0] - 2025-07-22

### Added
- **Format Declarations**:
  - Support for Perl `format` declarations
- **Modern Perl Features**:
  - Support for experimental class/method/field syntax
- **Edge Cases**:
  - Comprehensive coverage for tricky Perl syntax (globs, indirect objects)

## [v0.2.0] - 2025-07-22

### Added
- **Labeled Statements**: Support for loop labels and labeled blocks
- **Quote Operators**: Full support for `q`, `qq`, `qw`, `qr`, `qx` and `tr`/`y`
- **Bitwise Operators**: Parsing support for bitwise string operators

## [v0.1.0] - 2025-07-21

### Added
- **Pure Rust Parser**:
  - Initial release of the standalone Pure Rust Perl Parser
  - ~99.995% Syntax Coverage claimed
- **Heredoc Recovery**:
  - Dynamic heredoc recovery system

## Historical Releases

See [docs/archive/](docs/archive/) for older logs if available.
