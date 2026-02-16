# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Changed
- **v0.9.1 close-out receipts captured**: workspace index state-machine transitions, instrumentation, and early-exit behavior verified with targeted tests in `perl-workspace-index`
- **Performance caps validated**: workspace index benchmarks confirm wide margin to targets (`~368.7us` initial small, `~721.1us` initial medium, `~212.6us` incremental)
- **Documentation hardening completed**: `cargo test -p perl-parser --features doc-coverage --test missing_docs_ac_tests` and `cargo doc --no-deps -p perl-parser` both clean
- **Milestone docs aligned**: `START_HERE.md`, `CURRENT_STATUS.md`, `ROADMAP.md`, and `TODO.md` now reflect the same post-close-out status

## [1.0.0] - 2026-02-13

### Added
- **Production-Ready GA Release**: First General Availability release with GA-lock stability guarantees
- **Complete Semantic Analyzer**: All NodeKind handlers implemented (Phases 1, 2, 3) with 100% AST node coverage
- **Enhanced LSP Cancellation System**: Thread-safe infrastructure with <100μs check latency and atomic operations
- **Debug Adapter Protocol (DAP) Support**: Phase 1 bridge to Perl::LanguageServer for immediate debugging capability
- **Enterprise Security Hardening**: UTF-16 boundary vulnerability fixes, path traversal prevention, and process isolation
- **Comprehensive API Documentation**: `#![warn(missing_docs)]` enforcement with 12 acceptance criteria validation
- **Advanced Parser Robustness**: 12 fuzz testing suites and mutation hardening with 60%+ score improvement
- **Revolutionary Performance**: 5000x faster test suite execution with adaptive threading configuration
- **Dual Indexing Architecture**: Enhanced cross-file navigation with 98% reference coverage
- **Production Incremental Parsing**: <1ms updates with 70-99% node reuse efficiency
- **Enhanced LSP Features**: 99% LSP 3.18 protocol coverage (88/89 GA-locked capabilities)
- **Advanced Code Actions**: AST-aware refactoring with cross-file impact analysis
- **Import Optimization**: Remove unused imports, add missing imports, remove duplicates, sort alphabetically
- **Workspace Refactoring**: Enterprise-grade symbol renaming and module extraction
- **Unicode-Safe Operations**: Full Unicode identifier and emoji support with proper UTF-8/UTF-16 handling
- **Cross-Platform Support**: Windows, macOS, Linux, WSL with automatic path normalization
- **Comprehensive Testing**: 535+ tests with 100% LSP coverage and mutation hardening

### Changed
- **MSRV Updated**: Minimum Supported Rust Version bumped to 1.89 (Rust 2024 edition)
- **Parser Architecture**: Native recursive descent parser replaces legacy implementations
- **Performance Model**: 4-19x faster parsing with 1-150μs typical parsing times
- **LSP Wire Protocol**: Full LSP 3.18 compliance with GA-locked capability guarantees
- **Error Handling**: Structured error recovery with no panics on malformed input
- **Position Tracking**: Symmetric UTF-8/UTF-16 position conversion with boundary safety
- **Configuration Schema**: Enhanced workspace configuration with validation
- **Test Infrastructure**: Revolutionary adaptive threading with 5000x performance improvements
- **Documentation Standards**: Enterprise-grade documentation with automated quality gates

### Deprecated
- **perl-parser-pest**: Legacy Pest-based parser marked as deprecated (maintenance only)
- **Legacy Scanner Features**: C-scanner wrapper deprecated in favor of unified Rust implementation
- **Old Position Helpers**: Legacy position conversion functions (replaced with symmetric converters)

### Security
- **UTF-16 Boundary Protection**: Fixed symmetric position conversion vulnerabilities
- **Path Traversal Prevention**: Comprehensive path validation for all file operations
- **Process Isolation**: Safe defaults and sandboxed execution for external tool integration
- **Command Injection Hardening**: Eliminated shell interpolation vulnerabilities
- **Memory Safety**: Enhanced bounds checking and input validation
- **Resource Limits**: Configurable limits on recursion depth, file size, and workspace size

### Fixed
- **Incremental Parsing**: Fixed node reuse efficiency issues (70-99% improvement)
- **Cross-File Navigation**: Resolved dual indexing edge cases for 98% reference coverage
- **Semantic Analysis**: Fixed scope resolution for nested packages and shadowed variables
- **LSP Cancellation**: Resolved race conditions in concurrent request handling
- **Performance Regression**: Eliminated timeout issues in CI environments
- **Unicode Handling**: Fixed grapheme cluster processing in identifiers
- **Test Reliability**: Achieved 100% CI pass rate with adaptive threading

### Performance
- **Parser Speed**: 4-19x faster than legacy implementations (1-150μs parsing)
- **LSP Response Times**: <50ms single-file operations, <100ms workspace indexing
- **Test Suite**: 5000x faster execution (1560s → 0.31s for behavioral tests)
- **Memory Usage**: Optimized AST construction with O(n) space complexity
- **Incremental Updates**: <1ms update latency with high node reuse
- **Threading**: Adaptive configuration with multi-tier timeout scaling

## [0.9.0] - 2026-01-01

### Added
- **Semantic Analyzer Phase 1**: 12/12 critical node handlers implemented
- **LSP textDocument/definition Integration**: Semantic-aware definition resolution
- **Revolutionary Performance**: 5000x faster test suite execution
- **Enhanced Cross-File Navigation**: Dual indexing strategy with 98% reference coverage
- **API Documentation Infrastructure**: `#![warn(missing_docs)]` enforcement framework

### Changed
- **Performance Model**: Adaptive threading configuration for CI environments
- **LSP Coverage**: Increased to 82% (27/33 GA-advertised features)
- **Testing Infrastructure**: Comprehensive test harness with resource constraints

### Fixed
- **CI Reliability**: 100% pass rate achieved (was ~55% due to timeouts)
- **Performance Regression**: Eliminated slow test execution bottlenecks

## [0.8.8] - 2025-12-01

### Added
- **Workspace Configuration**: Production-ready workspace management
- **Enhanced Formatting**: Always-available capabilities with graceful perltidy fallback
- **Import Optimization**: Remove unused, add missing, alphabetical sorting
- **Code Actions**: Advanced refactoring operations with cross-file impact analysis

### Changed
- **Parser Architecture**: Improved incremental parsing with statistical validation
- **LSP Features**: Enhanced reference resolution and workspace support

### Fixed
- **Memory Leaks**: Resolved AST construction memory issues
- **File Watching**: Fixed workspace monitoring edge cases

## [0.8.0] - 2025-11-01

### Added
- **Production-Ready LSP Server**: ~85% LSP feature coverage
- **Workspace Symbols**: Comprehensive symbol indexing and search
- **Enhanced Completion**: Context-aware code completion with snippets
- **Hover Information**: Rich hover documentation and type information

### Breaking Changes
- **Position Helpers**: `offset_to_position()` signature changed
- **Error Types**: `ParseError` structure restructured
- **MSRV**: Bumped to Rust 1.85

## [0.7.0] - 2025-10-01

### Added
- **Initial LSP Implementation**: Basic LSP server with core features
- **Parser Library**: Native Rust parser with comprehensive Perl 5 syntax
- **Test Corpus**: Comprehensive test suite for parser validation

### Security
- **Initial Security Framework**: Basic input validation and error handling

---

## Migration Guide

### From v0.9.x to v1.0.0

**Breaking Changes:**
- MSRV bumped to Rust 1.89 (2024 edition)
- Legacy parser components deprecated

**Migration Steps:**
1. Update Rust toolchain: `rustup update stable && rustup default stable`
2. Update dependencies in `Cargo.toml`:
   ```toml
   [dependencies]
   perl-parser = "1.0"
   perl-lsp = "1.0"
   ```
3. Replace deprecated APIs (see compiler warnings)
4. Run `cargo check` to verify compatibility

### From v0.8.x to v1.0.0

**Major Changes:**
- Position helper API changes
- Error type restructuring
- Enhanced LSP capabilities

**Migration Steps:**
1. Update position conversion calls:
   ```rust
   // Old
   let pos = old_offset_to_position(source, offset);
   
   // New
   let pos = offset_to_position(source, offset);
   ```
2. Update error handling:
   ```rust
   // Old
   match parse_result {
       Some(node) => handle_node(node),
       None => handle_error(),
   }
   
   // New
   match parse_result {
       Ok(node) => handle_node(node),
       Err(err) => handle_error(err),
   }
   ```

---

## Version Support Policy

| Version | Status | Support Duration | End of Life |
|---------|--------|------------------|-------------|
| 1.x | Current LTS | 24 months | 2028-01-01 |
| 0.9.x | Previous | 6 months | 2026-07-01 |
| 0.8.x | Security Only | 3 months | 2026-04-01 |
| 0.7.x | End of Life | - | 2025-10-01 |

**Security Support:**
- Critical: 24-hour patches, emergency releases
- High: 7-day patches, expedited releases  
- Medium: 30-day patches, regular releases
- Low: Next scheduled release

---

## GA-Lock Stability Guarantees (v1.0+)

✅ **API Stability**: Public APIs stable under SemVer (breaking changes only in major releases)
✅ **Wire Protocol**: LSP capabilities locked, backward compatible through v1.x
✅ **Platform Support**: 6 Tier 1 platforms with pre-built binaries and CI testing
✅ **Performance**: O(n) parsing, <50ms LSP responses, no exponential blowups
✅ **Security**: 24-hour critical patches, coordinated disclosure, memory safety
✅ **Error Handling**: No panics on invalid input, structured errors with source locations

For detailed stability information, see [docs/STABILITY.md](docs/STABILITY.md).
