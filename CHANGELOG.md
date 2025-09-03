# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added
- **perl-lsp Crate Separation (PR #12)** - Comprehensive LSP crate separation with production-ready architecture:
  - Standalone `perl-lsp` crate for LSP binary with clean CLI interface
  - Enhanced modularity with clear separation between parser and LSP functionality
  - Production-grade command-line options (`--health`, `--features-json`, `--version`)
  - Improved maintainability and independent versioning capabilities
  - Enhanced testing architecture with dedicated LSP integration test suite
- **Import Optimization** - Comprehensive analysis and optimization of Perl import statements
  - Unused import detection with regex-based usage analysis
  - Duplicate import consolidation across multiple lines  
  - Missing import detection for Module::symbol references (planned)
  - Optimized import generation with alphabetical sorting
  - Complete test coverage with 9 comprehensive test cases
- **Built-in Function Parsing Enhancement** - Fixed 15 test failures in builtin_empty_blocks_test.rs
- **Architectural Quality Improvements** - Zero clippy warnings, consistent formatting across all crates

### Changed
- **Crate Structure** - LSP server binary moved from perl-parser to dedicated perl-lsp crate
- **Installation Method** - `cargo install perl-lsp` now installs from dedicated crate instead of perl-parser
- **Development Workflow** - Clear separation of parser improvements vs LSP binary enhancements

### Improved
- **Code Organization** - Clean architectural boundaries between parsing logic and LSP protocol implementation
- **Distribution Strategy** - Users can install only what they need (library vs binary)
- **Testing Coverage** - 235+ tests passing across production crates with enhanced reliability
- **Documentation** - Updated installation guides, tutorials, and architectural explanations

## [v0.8.9] - 2025-09-03

### Added  
- **Comprehensive PR Workflow Integration** - Production-stable AST generation and enhanced workspace navigation
- **Enhanced Workspace Features** - Improved AST traversal including `NodeKind::ExpressionStatement` support across all providers
- **Advanced Code Actions and Refactoring** - Fixed parameter threshold validation and enhanced refactoring suggestions
- **Enhanced Call Hierarchy Provider** - Complete workspace analysis with improved function call tracking
- **Production-Ready Reliability** - 100% test pass rate (195/195 library, 33/33 LSP E2E, 19/19 DAP tests)

### Improved
- **File Path Completion** - Enterprise-grade security with path traversal prevention and 18 comprehensive tests
- **Cross-File Navigation** - Enhanced workspace indexing with comprehensive symbol tracking
- **Quality Assurance** - Zero clippy warnings, consistent formatting, full architectural compliance

## [v0.8.8] - 2025-08-01

### Added
- **Advanced Incremental Parsing V2** - Revolutionary incremental parser with intelligent node reuse and detailed metrics tracking
- **Smart Node Reuse Strategy** - Automatically detects which AST nodes can be preserved across edits for optimal performance
- **Comprehensive LSP Integration** - Incremental parsing integrated with LSP server via feature flags and environment variables
- **Tree-sitter Grammar Enhancement** - Added given/when/default grammar rules for complete switch-style control flow support
- **Enhanced Control Flow** - Tree-sitter grammar now supports all modern Perl control flow constructs
- **Comprehensive Corpus Testing** - Added test corpus for given/when/default constructs with edge case coverage
- **IncrementalParserV2 Example** - Added comprehensive example demonstrating incremental parsing capabilities with metrics

### Improved
- **Incremental Performance** - Achieves 70-90% node reuse in typical editing scenarios with <1ms update times
- **Fallback Mechanisms** - Graceful degradation to full parsing when incremental optimizations aren't applicable
- **Feature Flag Architecture** - Clean separation of incremental features with `--features incremental` flag
- **Testing Coverage** - Added comprehensive test suite for incremental parsing with 6 integration tests passing
- **Parser Reliability** - Enhanced bless parsing capabilities with complete AST generation compatibility
- **Workspace Features** - Enhanced symbol extraction and workspace navigation improvements

### Changed
- **API Stability** - Maintained backward compatibility while adding new incremental parsing features
- **Performance Profile** - Incremental parsing now default-enabled for supported operations

### Fixed
- **Bless Parsing** - Resolved all bless parsing test failures with proper AST structure
- **Symbol Extraction** - Comprehensive AST traversal including ExpressionStatement nodes

## [v0.8.7] - 2025-08-01

### Added
- **Comprehensive Comment Documentation Extraction (PR #71)** - Production-ready leading comment parsing with extensive edge case coverage
- **Enhanced Source Threading Architecture** - Source-aware LSP providers with improved context for all features
- **20 Comprehensive Test Cases** - Complete test coverage for comment extraction including Unicode, performance, and edge cases
- **Multi-Package Comment Support** - Correct comment extraction across package boundaries with qualified name resolution
- **Class Method Documentation** - Support for extracting documentation from class methods and complex Perl constructs
- **Variable List Documentation** - Shared documentation for variable list declarations (`my ($a, $b, @c, %d)`)

### Improved
- **Performance Optimization** - Comment extraction optimized to <100µs per iteration with pre-allocated string capacity
- **Unicode Safety** - Proper UTF-8 character boundary handling for international comments and emojis  
- **S-Expression Format Compatibility** - Resolved bless parsing regressions with complete AST compatibility
- **Edge Case Robustness** - Handles empty comments, source boundaries, non-ASCII whitespace, and complex formatting
- **LSP Functionality** - Improved from 75% to 78% functional with enhanced documentation and symbol intelligence
- **Whitespace Handling** - Distinguishes between blank lines and whitespace-only lines for accurate comment boundaries

### Fixed
- **Bless Parsing Regression** - Resolved S-expression format issues affecting blessed object parsing
- **Comment Boundary Detection** - Precise handling of blank lines vs whitespace-only lines in comment extraction
- **Complex Formatting Scenarios** - Support for varying indentation, mixed hash styles, and special characters
>>>>>>> origin/master

## [v0.8.5] - 2025-08-24

### Added
- **Stable Diagnostic Codes** - All diagnostics now have stable codes (PL001-PL499, PC001-PC999) with optional documentation URLs
- **Pull Diagnostics Support** - Server detects client support and suppresses automatic publishing when client can pull
- **Capabilities Snapshot Testing** - Prevents unintentional capability drift with JSON snapshot tests
- **Typed Capabilities System** - Server capabilities are now generated from typed `BuildFlags` for consistency
- **Consolidated Builtin Signatures** - Single source of truth using perfect hash function (phf) for O(1) lookups
- **Test-Specific Slow Operation** - Added `$/test/slowOperation` endpoint for reliable cancellation testing

### Improved
- **Inlay Hints**
  - Enhanced `smart_arg_anchor` handles `@{...}`, `%{...}`, `&...` constructs properly
  - Correct anchoring for bareword filehandles and dereferencing expressions
  - Both parenthesized and non-parenthesized builtins show proper hints
  - Type hints for hash/array/ref variables are range-accurate
- **Cross-File Definition** - Improved scoring prioritizes same package > same file > others
- **Handler Gating** - Partially implemented features return method-not-found when not advertised
- **Position Handling** - Unified range checking via `pos_in_range()` and `pos_before()` utilities
- **Error Handling** - Consistent error responses via `lsp_errors` module
- **Cancellation Handling** - Eliminated race conditions in cancellation tests with predictable slow operation

### Fixed
- **Pull Diagnostics** - No double-flow when client supports pull diagnostics
- **Code Quality** - Fixed clippy warnings (clone-on-copy, explicit-counter-loop, useless-format, collapsible-if, manual-contains)
- **Test Infrastructure** - Fixed tautological test assertions in unit tests
- **Cancellation Tests** - All 3 cancellation tests now pass reliably without race conditions
- **Error Codes** - Standardized cancellation error code to -32802 across all handlers

### Infrastructure
- **Performance** - Builtin signatures use perfect hash for zero-allocation lookups
- **Architecture** - Cleaner separation with consolidated modules and single sources of truth
- **Safety** - Capability management with snapshot tests prevents regression
- **Logging** - Added diagnostic flow logging for debugging client interactions
- **Test Reliability** - Cancellation tests now use deterministic slow operation instead of racing

## [v0.8.4] - 2025-02-24

### Added
- **9 New LSP Features** - Transformed LSP from 35% to 60% functionality
  - Workspace symbol search with fuzzy matching across all open files
  - Cross-file rename refactoring (smart scoping for `our` vs `my` variables)
  - Code actions for missing pragmas (`use strict`, `use warnings`)
  - Semantic tokens for enhanced syntax highlighting
  - Inlay hints showing parameter names and type annotations
  - Document links from `use`/`require` statements to modules
  - Selection ranges for hierarchical smart selection
  - On-type formatting with auto-indent/dedent for braces
- **Contract-Driven Testing** - Every advertised capability backed by acceptance tests
- **Feature Flag Control** - `lsp-ga-lock` for conservative point releases
- **Fallback Mechanisms** - Graceful handling of incomplete/invalid code
- **530+ Tests** - Comprehensive E2E coverage for all LSP features

See detailed notes: [CHANGELOG_v0.8.4.md](CHANGELOG_v0.8.4.md)

### Fixed
- **LSP Server** - Removed duplicate method definitions
- **Declaration Module** - Fixed private function visibility
- **Semantic Tokens** - Fixed lexer iterator and token mapping issues
- **Integration Tests** - Updated to match new capability contract
- **Code Quality** - Fixed all clippy warnings in new modules

### Changed
- **LSP Architecture** - Clean module separation (one feature per file)
- **Error Handling** - Proper JSON-RPC error codes throughout
- **Memory Usage** - Arc-based AST with efficient parent maps
- **Position Mapping** - O(log n) UTF-16 conversions with LineStartsCache

### Previous Unreleased (Now in v0.8.4)
- **Lexer: Arithmetic underflows** - Fixed delimiter/substitution/brace depth counters using `saturating_sub()`
- **Lexer: Heredoc tokenization** - `<<...` now produces `TokenType::HeredocStart` (was `StringLiteral`)
- **Lexer: Sigil+brace handling** - `${`, `@{`, `%{` split into sigil and brace tokens when content is empty/invalid
- **Tests: Property test robustness** - Neighbor-aware whitespace insertion replaces fragile skip lists
- **Tests: Shared utilities** - Whitespace manipulation functions in `prop_test_utils.rs` for reuse
- **Tests: Lexer termination property** - Ensures lexer never panics or loops (256 cases)
- **Tests: Whitespace idempotence** - Verifies `respace_preserving` is idempotent
- **Tests: Parser integration** - Tests for new token types (heredoc starts, split sigil+brace)
- **CI: Property test workflow** - GitHub Actions for standard and extended property testing

## [v0.8.3] - 2025-08-21

### Added
- **Robust Heredoc Support** - Production-ready heredoc handling with all edge cases
  - FIFO multi-heredoc body ordering (matches Perl behavior exactly)
  - Indented heredocs (`<<~LABEL`) for Perl 5.26+ compatibility
  - Non-interpolating heredocs (`<<\LABEL`) with backslashed labels
  - BOM (UTF-8 byte order mark) handling at file start
  - CRLF line endings support throughout heredoc processing
  - Old Mac CR-only line endings (`\r` without `\n`) support
  - Unified helper functions for consistent CRLF/newline handling
  - Budget guards with graceful truncation (256KB max for heredoc bodies)
  - 22 comprehensive regression tests covering all edge cases

### Fixed
- **Code Quality Improvements**
  - Fixed all clippy warnings (manual range contains, unnecessary unwrap)
  - Fixed all tautological test assertions in LSP tests
  - Handle all Result types properly with expect() in test harness
  - Removed constant assertions that are always true
  - Consolidated CRLF handling with helper functions (~30 lines reduction)

### Changed
- **Heredoc Parser**
  - Stricter `__DATA__`/`__END__` parsing (requires line start, whitespace-only lines)
  - Improved error messages for heredoc edge cases
  - Added saturating arithmetic for budget guards to prevent underflow

## [0.8.2] - 2025-08-12

### Added
- **textDocument/documentLink** - MetaCPAN links for `use Module`, local files for `require`/`do`
  - Windows-safe URIs with proper percent-encoding
  - Handles paths with spaces and special characters
- **textDocument/selectionRange** - Smart hierarchical selection
  - identifier → expression → statement → block → function
- **textDocument/onTypeFormatting** - Auto-formatting on `{`, `}`, `)`, `;`, and newline
  - Smart indentation and brace alignment
- **workspace/didChangeWatchedFiles** - File watching with dynamic registration
  - Re-index on create/change/delete
  - External change synchronization

- **Incremental Parsing Infrastructure** - Production-ready incremental parsing support
  - High-performance incremental text updates using rope data structure
  - UTF-16 aware position conversion for full LSP protocol compliance
  - CRLF handling for Windows line endings compatibility
  - Dynamic capability advertisement (switches between FULL and INCREMENTAL modes)
  - Comprehensive test suite with 6 incremental tests + 4 E2E performance tests
  - Performance metrics tracking for monitoring improvements
  - SubtreeReuse capability foundation for future AST caching optimization
  - Activates with `PERL_LSP_INCREMENTAL=1` environment variable

- **Workspace Indexing Foundation** - Ready for v0.8.3 activation
  - Full symbol extraction across workspace
  - Cross-file symbol tracking and references
  - Module dependency tracking with thread-safe index
  - Integration with LSP server for workspace symbols
  - Robust percent-encoding for special characters in file paths
  - Handles spaces, Unicode, emojis, and Windows paths correctly
  - Clean API with `&str` interfaces throughout workspace modules

### Changed
- **LSP Server Architecture**
  - Made internals `pub(crate)` for incremental adapter integration
  - Created `DocumentParser` enum to support both full and incremental parsing modes
  - Enhanced position mapping with byte/UTF-16 conversion utilities
  - Position mapper with flexible UTF-8/UTF-16 handling
- **Test Infrastructure**
  - Robust LSP response handling with `completion_items()` helper
  - Handles both array and `{ items }` response formats
  - CI split: strict clippy for v3, compile-check for v2

### Quality
- **530+ tests passing** (increased from 526)
- **Zero clippy warnings** in v3 parser crates
- **100% edge case coverage** maintained

### Fixed
- **Test Infrastructure** – Rust 2024: `std::env::{set_var, remove_var}` are now `unsafe`; wrapped calls and ran env-mutating tests single-threaded
  - Fixed bless parsing tests expectations (removed incorrect `array` wrapper nodes)
  - Fixed incremental parsing test expectations (`Variable` vs `ScalarVariable` node names)
  - Serialized environment variable usage in tests with `EnvGuard` helper
  - Fixed workspace URI edge cases test API calls
  - Properly feature-gated incremental parsing handler

### Performance
- Parser benchmarks remain stable at ~8.5µs for simple scripts
- Workspace indexing overhead remains minimal
- Incremental parsing infrastructure adds no overhead when disabled
- All 78 test files passing with full feature set enabled

## [0.8.0] - Closed Beta

### ⚠️ Breaking Changes
- **Declaration Provider API** - Production-hardened with mandatory version tracking
  - **BREAKING**: `find_declaration()` now requires `current_version: i32` parameter
  - **BREAKING**: Must call `.with_doc_version()` after construction
  - Migration: Pass server's `doc_version` to `find_declaration(offset, col, doc_version)`
  
### Changed
- **Declaration Provider Safety** - Multiple protection layers
  - Guards against stale provider reuse after AST refresh  
  - Added `i32::MIN` sentinel to detect missing `with_doc_version()` calls
  - Parent map cycle detection in debug builds
  - Debug-only assertions with zero production overhead
  
### Improved
- **Position Conversion Performance** - 40-100x speedup on large files
  - Cached O(log n) binary search replaces O(n) scan
  - Test-only helpers marked with clear boundaries for CI validation
  - Cross-platform CI checks (Linux/macOS/Windows) for regression prevention
  
### Added
- **Comprehensive Unicode Tests** - Edge case coverage
  - ZWJ sequences (emoji families with >2 UTF-16 units)
  - BOM handling at file start
  - Column clamping on extremely long lines (20k+ chars)
  - Property tests verifying cache matches slow path exactly

## [0.7.5] - Closed Beta

### 🚀 Release Infrastructure
- **Enterprise-grade Distribution** - Professional release automation
  - cargo-dist configuration for multi-platform binary releases
  - Automated builds for Linux/macOS/Windows (x86_64/aarch64)
  - SHA256 checksums for all release artifacts
  - GitHub Actions release workflow with automatic publishing

- **Comprehensive CI/CD Pipeline** - Quality gates and automation
  - Test matrix across Linux, macOS, Windows
  - Static analysis with clippy and rustfmt
  - Code coverage reporting with tarpaulin
  - Benchmark regression detection
  - Test discovery guard (requires >100 tests)
  - PR comment with benchmark comparisons

- **Easy Installation Methods** - Multiple distribution channels
  - One-liner installer script with checksum verification
  - Homebrew formula for macOS users
  - Pre-built binaries for all major platforms
  - Smart PATH detection and shell config updates
  - Robust temp directory extraction handling

### ✨ Enhanced Type System
- **Smart Hash Type Inference** - Improved IDE intelligence
  - Intelligent type unification for hash literals
  - Homogeneous value detection (e.g., all integers → Hash<String, Integer>)
  - Mixed type handling with Union types
  - Recursive unification for nested structures
  - Full test coverage for inference scenarios

### Critical Fix
- **Test Infrastructure** - Recovered 400+ silently skipped tests
  - Fixed wrapper issue that incorrectly passed shell redirections as test filters
  - Tests now properly discovered when run in normal shells
  - Added comprehensive test runner script with --list based verification
  - Real test count: 526+ tests (was showing only 27)

### Added
- **Workspace File Operations Support** - Complete file synchronization
  - `workspace/didChangeWatchedFiles` - Re-indexes files changed externally
  - `workspace/willRenameFiles` - Updates module references when files are renamed
  - `workspace/didDeleteFiles` - Removes deleted files from workspace index
  - `workspace/applyEdit` - Applies multi-file refactoring operations
  - Automatic module name extraction from file paths
  - Smart import updating (use, require, use parent, use base)
  - Robust error handling with graceful fallbacks

- **Document Highlight Provider** - Highlight all occurrences of symbols
  - Highlights variables ($foo, @arr, %hash), functions, and methods
  - Smart exact matching (e.g., $foo won't highlight $food)
  - Efficient single-pass AST traversal
  - Full LSP integration with proper capability advertisement

- **Type Hierarchy Provider** - Navigate inheritance relationships
  - Find supertypes (parent classes) via @ISA and use parent/base pragmas
  - Find subtypes (child classes) that inherit from a class
  - Proper package scope tracking for linear and block form packages
  - Support for multiple inheritance patterns:
    - `use parent 'Base'` and `use base 'Base'`
    - `our @ISA = ('Base')` and `our @ISA = qw(Base1 Base2)`
    - Bareword lists: `@ISA = (Base)`
    - All quote styles: single, double, backticks
  - Handles namespaced packages (My::Base::Class)
  - Comprehensive qw() parsing with various delimiters

- **Compatibility Shim** - Zero-cost API migration layer
  - Added `src/compat.rs` module for smooth test migration
  - Feature-gated behind `test-compat` flag
  - All functions marked deprecated to track migration progress
  - Enables old tests to run while transitioning to new API

### Improved
- **Error Handling** - Replaced critical `.unwrap()` calls
  - 8+ unwrap calls in workspace handlers replaced with proper error handling
  - Added let-else patterns for cleaner error flow
  - Improved robustness for production environments

- **CI/CD** - Added test discovery guards
  - New workflow verifies no test files have 0 tests
  - Comprehensive test runner script in `.github/run_all_tests.sh`
  - Prevents regression of test discovery issues
  - Documents minimum expected test counts

### Tests
- **Test Discovery Fixed** - All 526+ tests now running
  - 400+ LSP integration tests recovered
  - 126 library unit tests
  - 13 new workspace file operations tests
  - All tests properly discovered with empty filter workaround
  - File creation, modification, and deletion handling
  - Module rename and import updating
  - Multi-file edit application
  - Error cases with missing URIs and invalid parameters

## [0.7.4] - Closed Beta

### Fixed
- **Test Infrastructure** - Complete overhaul of test assertions
  - Fixed 27+ tautological assertions that were always passing
  - Created centralized test support module with production-grade helpers
  - Deep validation of all LSP response structures
  - Meaningful error messages for debugging test failures

### Improved
- **Code Quality** - Zero compilation warnings
  - Removed 159+ lines of obsolete diagnostics code
  - Properly marked all intentionally unused stub implementations
  - Cleaned up dead code in WorkspaceIndex, TypeInferenceEngine, etc.
  - All 33 comprehensive tests passing (25 E2E + 8 user story tests)

### Added
- **Test Support Module** (`tests/support/mod.rs`)
  - `assert_hover_has_text()` - Validates hover content and range
  - `assert_completion_has_items()` - Ensures items with labels
  - `assert_references_found()` - Validates URI and range for each ref
  - `assert_call_hierarchy_items()` - Checks name, URI, range/selection
  - `assert_workspace_symbols_valid()` - Requires location or container
  - `assert_code_actions_available()` - Validates title + command/edit
  - `assert_folding_ranges_valid()` - Ensures multi-line spans

## [0.7.3] - Closed Beta

### Added
- **Incremental Parsing** - Lightning-fast incremental updates with <1ms performance
  - IncrementalDocument with full subtree reuse and caching
  - Smart optimizations for single-token updates
  - Content-based caching with LRU management
  - Performance: 0.005ms average update time (200x better than target!)
  - High subtree reuse rate for efficient editing

- **Workspace-wide Refactoring** - Professional refactoring capabilities across entire projects
  - WorkspaceIndex for cross-file symbol tracking and dependency management
  - Multi-file rename refactoring with reference tracking
  - Extract module/package refactoring
  - Move subroutine between files
  - Import optimization (detect unused, missing, duplicate imports)
  - Architecture ready for full AST traversal

- **Dead Code Detection** - Comprehensive dead code analysis
  - Find unused subroutines, variables, and packages
  - Detect unreachable code after return statements
  - Identify dead branches in conditionals
  - Cross-file analysis with confidence levels
  - Smart filtering for test files and entry points

- **Type Inference Foundation** - Basic type inference system for future enhancements
  - Infer types for variables from usage patterns
  - Built-in function type signatures for 150+ functions
  - Type-based code completions
  - Type constraint checking
  - Foundation for advanced static analysis

- **Perl::Critic Integration** - Static code analysis and policy enforcement
  - Support for all 5 severity levels (Gentle to Brutal)
  - Configurable policy profiles
  - Quick fixes for common violations
  - Built-in policies for when perlcritic is not available
  - Cache for improved performance

- **Perltidy Integration** - Automatic code formatting
  - Full perltidy configuration support
  - PBP and GNU style presets
  - Range formatting capability
  - Built-in formatter for when perltidy is not available
  - Formatting suggestions without applying changes

- **TDD Workflow Support** - Test-Driven Development features
  - Automatic test generation for subroutines
  - Support for Test::More, Test2::V0, Test::Simple, Test::Class
  - Red-green-refactor cycle management
  - Cyclomatic complexity analysis for refactoring suggestions
  - Method length analysis (>50 lines triggers suggestions)
  - Parameter count analysis (>5 parameters triggers suggestions)
  - Coverage tracking and diagnostics integration

### Improved
- **Code Quality**: Fixed 81 clippy warnings across perl-lexer and perl-parser (61% reduction)
  - Eliminated 45+ unnecessary `.clone()` calls on Copy types for better performance
  - Fixed recursive function warnings with proper annotations
  - Replaced `.get(0)` with `.first()` for better Rust idioms
  - Changed `push_str("x")` to `push('x')` for single characters
  - Replaced `format!("literal")` with `"literal".to_string()` where appropriate
  - Fixed `or_insert_with(Vec::new)` to use `or_default()` for cleaner code
  - Removed unnecessary borrows from `format!()` expressions
  - Overall memory usage and performance improvements from avoiding unnecessary allocations

## [0.7.2] - Closed Beta

### Fixed
- **Parser**: Fixed incorrect operator precedence for word operators (`or`, `and`, `not`, `xor`)
  - These operators now correctly have lower precedence than assignment operators
  - `$a = 1 or $b = 2` now parses as `($a = 1) or ($b = 2)` instead of `$a = (1 or $b) = 2`
  - Fixes long-standing issue where word operators were incorrectly parsed with higher precedence
  - Added comprehensive test suite with 10 test cases for word operator precedence
- **Parser**: Fixed division operator (`/`) parsing
  - Division operator was not being recognized by the token stream converter
  - Expressions like `$a / $b` and `10 / 2` now parse correctly
  - The lexer was correctly producing Division tokens but they weren't being converted to Slash tokens for the parser

### Added
- **LSP**: Complete built-in function signatures for all 150+ Perl built-in functions
  - Added comprehensive signature information for string, array, hash, I/O, file, directory, process, time, math, and socket functions
  - Each function includes multiple signature variations and documentation
  - Functions now covered include: `chomp`, `chop`, `chr`, `ord`, `hex`, `oct`, `length`, `lc`, `uc`, `ucfirst`, `lcfirst`, `quotemeta`, `index`, `rindex`, `sprintf`, `say`, `read`, `sysread`, `write`, `syswrite`, `seek`, `tell`, `eof`, `stat`, `lstat`, `chmod`, `chown`, `link`, `symlink`, `readlink`, `rename`, `unlink`, `mkdir`, `rmdir`, `opendir`, `readdir`, `closedir`, `fork`, `wait`, `waitpid`, `kill`, `getpid`, `getppid`, `time`, `localtime`, `gmtime`, `sleep`, `alarm`, `abs`, `atan2`, `cos`, `sin`, `exp`, `log`, `sqrt`, `int`, `rand`, `srand`, `scalar`, `wantarray`, `caller`, `eval`, `do`, `tie`, `tied`, `untie`, `socket`, `bind`, `listen`, `accept`, `connect`, `shutdown`, `send`, `recv`, `pack`, `unpack`, `study`, `pos`, `reset`, `vec`, `prototype`, `lock`, and many more
  - Improves IDE experience with accurate parameter hints and documentation

## [0.7.1] - Closed Beta

### Fixed
- **Parser**: Fixed incorrect parsing of `bless {}` syntax which was being treated as hash element access instead of a function call with empty hash argument
  - Now correctly parses `bless {}` as `(call bless ((hash)))`
  - Fixes work in all contexts: statements, returns, nested in subroutines
  - Added comprehensive test coverage for all `bless` variations
- **Parser**: Fixed parsing of empty blocks in `sort {}`, `map {}`, and `grep {}`
  - Previously failed on empty blocks, now correctly parses as `(call sort ((block)))`
  - Added support for both empty blocks and blocks with expressions
  - Added 15 new test cases covering various builtin functions with empty arguments

## [0.7.0] - Closed Alpha

### Next Release Planning
- Debugger Adapter Protocol (DAP) support
- Live Share collaboration features
- Remote development support
- Custom snippets system
- Perl::Critic integration

## [0.6.0] - Closed Alpha

### 🎉 Production-Ready LSP with Comprehensive Testing

This release marks a major milestone with comprehensive end-to-end testing, making the LSP truly production-ready for enterprise use.

### Added

#### Comprehensive Test Suite (NEW - January 29, 2025)
- **63+ User Story Tests** - Real-world IDE workflows
  - Core LSP features (11 tests)
  - Built-in functions (9 tests, 114 functions)
  - Edge cases (13 tests)
  - Multi-file support (6 tests)
  - Testing integration (6 tests)
  - Refactoring (6 tests)
  - Performance (6 tests)
  - Formatting (7 tests)
- **Master Integration Test** - Validates entire LSP lifecycle
- **Test Fixtures** - Real Perl project structure for testing
- **CI/CD Pipeline** - GitHub Actions for automated testing
- **Release Automation** - Scripts for versioning and publishing
- **VSCode Extension Manifest** - Complete extension configuration
- **Coverage Reporting** - 95% user story coverage achieved

#### Advanced IDE Features (from v0.5.0)
- 🔍 **Call Hierarchy** - Navigate function relationships
  - View incoming calls (who calls this function)
  - View outgoing calls (what this function calls)
  - Support for both functions and methods
  - Right-click context menu integration
- 💡 **Inlay Hints** - Inline parameter and type information
  - Parameter name hints for function calls
  - Type hints for variable declarations
  - Smart filtering to avoid clutter
  - Fully configurable (enable/disable by type)
- 🧪 **Test Runner Integration** - Run tests from VSCode
  - Automatic test discovery for .t files
  - Test Explorer panel integration
  - Run individual tests or entire files
  - TAP (Test Anything Protocol) support
  - Real-time test results with pass/fail indicators
- ⚙️ **Configuration Options**
  - Inlay hints: enable/disable, parameter/type hints, max length
  - Test runner: command, arguments, timeout settings
  - All features configurable via VSCode settings

### Performance
- Parser improvements: 100% edge case coverage maintained
- Efficient AST traversal for feature extraction
- Optimized inlay hint filtering

## [0.5.0] - Internal Alpha

### 🚀 Major Release: Complete LSP Implementation with VSCode Extension

This release delivers a production-ready Language Server Protocol implementation that transforms Perl development with modern IDE features.

### Added
- 🎯 **Workspace Symbols** - Project-wide symbol search with fuzzy matching (Ctrl+T)
  - Real-time indexing of all open documents
  - Fuzzy search algorithm for quick navigation
  - Support for packages, subroutines, constants, and variables
- 🏃 **Code Lens** - Inline actions above code elements
  - "▶ Run Test" for test functions (test_*, Test*, *_test patterns)
  - "X references" for subroutines and packages
  - "▶ Run Script" for files with shebang
  - Lazy resolution for performance
- 📦 **VSCode Extension** - One-click installation
  - Complete language client implementation
  - Enhanced TextMate grammar for syntax highlighting
  - Bundled LSP binary (1.5MB)
  - Cross-platform support (Windows, macOS, Linux)
- 🧪 **Comprehensive Test Suite**
  - 9 LSP integration tests
  - Workspace symbols tests
  - Code lens provider tests
  - Multi-document handling tests
- 🚀 **Full Language Server Protocol (LSP) implementation**
  - Syntax diagnostics with real-time error detection
  - Symbol navigation (go to definition, find references)
  - Document symbols for outline view
  - Signature help for function parameters
  - Code completion with context awareness
  - Hover information with type details
  - Document formatting with Perl::Tidy
  - Code actions and quick fixes
  - Incremental parsing for efficient updates
- ✅ **Error recovery parser** for better IDE experience
- ✅ **Trivia preservation** for comments and whitespace

### Changed
- Enhanced LSP server architecture with modular feature providers
- Improved symbol extraction with better categorization
- Optimized workspace indexing for large projects
- Updated documentation to reflect new features:
  - Added comprehensive LSP documentation suite
  - Created feature roadmap (FEATURE_ROADMAP.md)
  - Added LSP implementation examples
  - Enhanced best practices guide
  - Updated README with quick start guide

### Fixed
- Fixed compilation errors in refactoring module
- Resolved symbol matching case sensitivity issues
- Fixed code lens position calculations
- Improved error handling in LSP request processing
- Fixed private method visibility in workspace symbols

## [0.4.0] - Internal Alpha

### 🎉 v3 Parser Complete - 100% Edge Case Coverage

This release marks the completion of the v3 native parser (perl-lexer + perl-parser) with full Perl 5 syntax support.

### Added
- ✅ **Underscore prototype support** (`sub test(_) { }`)
- ✅ **Defined-or operator** (`//`) 
- ✅ **Glob dereference** (`*$ref`)
- ✅ **Pragma arguments** (`use constant FOO => 42`)
- ✅ **List interpolation** (`@{[ expr ]}`)
- ✅ **Multi-variable attributes** (`my ($x :shared, $y :locked)`)
- ✅ **Indirect object syntax** (`print STDOUT "hello"`)
- ✅ Complete Tree-sitter compatibility documentation
- ✅ Syntax highlighting queries (`queries/highlights.scm`)
- ✅ Format transformation utilities
- ✅ S-expression analysis examples

### Changed
- Updated all documentation to reflect 100% edge case coverage
- Improved parser performance for complex expressions
- Enhanced Tree-sitter S-expression output format

### Fixed
- Fixed operator precedence for defined-or (`//`)
- Fixed tokenization of underscore in prototypes
- Fixed pragma argument parsing
- Fixed multi-variable attribute parsing

### Performance
- v3 parser: 4-19x faster than v1 (C implementation)
- Simple files: ~1.1 µs
- Medium files: ~50 µs
- Large files: ~150 µs

### Statistics
- **Edge case tests**: 141/141 passing (100%)
- **Perl 5 coverage**: ~100%
- **Dependencies**: Zero

## [0.3.0] - Internal Alpha

### Added
- Initial v3 parser implementation (perl-lexer + perl-parser)
- Context-aware lexing for slash disambiguation
- Recursive descent parser with operator precedence
- Modern Perl features (class, method, try/catch)
- Unicode identifier support
- Comprehensive edge case test suite

### Performance
- Achieved 4-19x speedup over C implementation
- Benchmarking infrastructure for all three parsers

## [0.2.0] - Internal Alpha

### 🎉 Major Improvements: Edge Case Coverage Increased to 94.5%

### Added
- **Deep dereference chains** - Full support for complex chains like `$hash->{key}->[0]->{sub}`
- **Double quoted string interpolation** - Proper parsing of `qq{hello $world}` with variable detection
- **Postfix code dereference** - Support for `$ref->&*` syntax for dereferencing code references
- **Keywords as identifiers** - Reserved words can now be used as method names and in expressions

### Fixed
- Fixed parsing of deeply nested dereference chains that previously failed
- Fixed `qq{}` operator to properly handle interpolated variables
- Fixed postfix dereference syntax for code references
- Fixed keyword handling in method calls and expressions

### Changed
- **Edge case coverage improved from 82.8% to 94.5%** - Significant increase in parser robustness
- Enhanced parser to handle more complex Perl idioms
- Improved error recovery for edge cases

### Remaining Edge Cases (7)
The following edge cases still need implementation:
1. **Labels** - `LABEL: for (@list) { }` - requires proper lookahead
2. **Subroutine attributes** - `sub bar : lvalue { }`
3. **Variable attributes** - `my $x :shared`
4. **Format declarations** - `format STDOUT =`
5. **Class declarations** - `class Foo { }` (Perl 5.38+)
6. **Method declarations** - `method bar { }` (Perl 5.38+)

### Test Results
- **94.5% edge case coverage** - Major improvement from previous 82.8%
- All new features have comprehensive test coverage
- Performance characteristics maintained (~180 µs/KB)

## [0.1.0] - Internal Alpha

### 🎉 Major Milestone: 99.995% Perl 5 Coverage

### Added
- **Reference operator (`\`)** - Full support for creating references (`\$scalar`, `\@array`, `\%hash`, `\&sub`)
- **Modern octal format** - Support for `0o755` notation alongside traditional `0755`
- **Ellipsis operator (`...`)** - Proper tokenization of the yada-yada operator
- **Enhanced edge case handling** - Now passing all 15 edge case tests (100% coverage)
- **Improved lexer architecture** - Better handling of compound operators

### Fixed
- Fixed typeglob slot syntax parsing (`*foo{SCALAR}`)
- Fixed operator overloading syntax (`use overload '+' => \&add`)
- Fixed unreachable pattern warning in lexer
- Fixed octal number parsing for modern format

### Changed
- **Coverage improved from ~99.99% to ~99.995%**
- Updated all documentation to reflect new coverage metrics
- Enhanced Unicode identifier support (already working, now with comprehensive tests)

### Edge Cases Now Supported
1. Format strings (`format STDOUT = ...`)
2. V-strings (`v1.2.3`)
3. Stacked file tests (`-f -w -x $file`)
4. Array/hash slices (`@array[1,2]`, `@hash{qw/a b/}`)
5. Complex regex features (`(?{ code })`, `(?!pattern)`)
6. Encoding pragmas (`use encoding 'utf8'`)
7. Multi-character regex delimiters (`s### ###`)
8. Symbolic references (`$$ref`, `*{$glob}`)
9. `__DATA__` section handling
10. Indirect object syntax (`new Class @args`)
11. Reference operator (`\$scalar`)
12. Underscore special filehandle (`_`)
13. Operator overloading (`use overload`)
14. Typeglob slots (`*foo{SCALAR}`)
15. `AUTOLOAD` method support

### Test Results
- **100% edge case coverage** - All 15 edge case tests passing
- **All features verified** - Reference operator, modern octal, ellipsis, Unicode
- **Tree-sitter compatibility** - S-expression output confirmed working
- **Performance validated** - ~180 µs/KB as documented

### Known Limitations
- **Heredoc-in-string** (~0.005% impact) - Heredocs initiated from within interpolated strings (`"$prefix<<$end_tag"`)

---

## [0.0.1] - 2024-12-XX - Initial Pure Rust Parser

### 🚀 Major Achievement
- **Pure Rust Perl Parser** built with Pest parser generator
- **~99.99% Perl 5 syntax coverage** - handles virtually all real-world Perl code
- **Tree-sitter compatible** S-expression output
- **Zero C dependencies** - 100% pure Rust implementation
- **Excellent performance** - ~200-450 µs for typical files (~180 µs/KB)

### ✨ Core Features
- **Complete Perl 5 Support**:
  - All variable types (scalar, array, hash) with all declaration types
  - Full string interpolation (`$var`, `@array`, `${expr}`)
  - Regular expressions with all operators and modifiers
  - 100+ operators with correct precedence
  - All control flow constructs
  - Subroutines with signatures and type constraints (Perl 5.36+)
  - Modern Perl features (try/catch, defer, class/method)
  - Advanced heredocs with complete edge case handling
  - Full Unicode support including identifiers
  
### 🔧 Technical Implementation
- **Pest Parser** - PEG-based grammar in `grammar.pest`
- **Context-sensitive parsing** - Slash disambiguation, heredoc handling
- **Multi-phase parsing** - Handles stateful constructs like heredocs
- **Edge case recovery** - Comprehensive error handling and recovery
- **Memory efficient** - Arc<str> for zero-copy string storage
- **Cross-platform** - Linux, macOS, and Windows support

### 🧪 Testing & Quality
- **Comprehensive test suite** with 16+ test files
- **Edge case test suite** - 14/15 tests passing (93% coverage)
- **Property-based testing** for robustness
- **Performance benchmarks** with consistent results
- **Integration tests** for tree-sitter compatibility

### 📚 Documentation
- **Complete feature documentation** in FEATURES.md
- **Known limitations** clearly documented in KNOWN_LIMITATIONS.md
- **Architecture guide** for understanding the implementation
- **Edge case documentation** with detailed explanations
- **Development guide** for contributors

---

## [Evaluation] - Legacy C Implementation

### 🎉 Initial Release
- Initial tree-sitter Perl parser implementation
- C-based scanner with JavaScript grammar
- Support for Neovim and Emacs tree-sitter integration
- Comprehensive corpus test suite

### ✨ Features
- Full Perl syntax parsing support
- Heredoc and quote handling
- Unicode identifier support
- Complex interpolation logic
- POD documentation parsing

### 🔧 Technical
- C scanner implementation (`src/scanner.c`)
- JavaScript grammar definition (`grammar.js`)
- Multi-language bindings (Node.js, Rust, Python, Go, Swift)
- Tree-sitter corpus test coverage

---

## Migration Notes

### For Users Upgrading to Pure Rust Parser
1. **No API changes** - S-expression output remains compatible
2. **Better performance** - Expect 2-3x improvement in parsing speed
3. **Enhanced coverage** - More edge cases handled correctly
4. **Pure Rust** - No C toolchain required for building

### Breaking Changes
None - The Pure Rust parser maintains full compatibility with the C implementation

### Upgrade Guide
1. **Update dependencies**: Use the pure-rust feature flag
2. **Build with**: `cargo build --features pure-rust`
3. **Test thoroughly**: Verify your specific use cases work correctly

---

*For detailed architecture information, see [ARCHITECTURE.md](./ARCHITECTURE.md)*  
*For development guidelines, see [CONTRIBUTING.md](./CONTRIBUTING.md)*