# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.10.0] - 2026-02-28

### Added
- **Document Highlight Improvements**: Support for modern Perl syntax (try/catch parameters, method/sub signatures, string interpolation) (#882).
- **CI Workflow Hardening**: Concurrency groups to prevent duplicate release runs, fixed asset URLs, scoop/chocolatey packaging fixes (#890).
- **Semver-Aware Benchmark Sorting**: Correct version comparison for benchmark baseline selection (#885).
- **crates.io Publishing Readiness**: All crate metadata verified, publish-ignore lists normalized, crate badges added (#871).
- **Module Infrastructure Crates**: Content-Length framing and LSP hardening (#857).
- **Feature Governance Microcrates**: Extracted into 9 dedicated crates (#848).
- **InlineValues Lifecycle Coverage**: Added test coverage for inlineValues support (#729).
- **Context-Aware Status Menu**: Improved UX with context-aware states (#646).

### Changed
- **Version Consistency**: All workspace crates, documentation, VS Code extension, and feature catalogs updated to v0.10.0 (77+ files).
- **Documentation Refresh**: Updated editor setup guides, roadmap, milestones, and stability policy for v0.10.0.
- **features.toml**: Version bumped to 0.10.0 with 100% LSP coverage maintained (53/53 user-visible, 97/97 protocol).
- **VS Code Extension**: Version synchronized to 0.10.0 with packaging fixes (#863, #866, #869).

### Fixed
- **Build Fixes**: Resolved 4 compilation errors in the release candidate build (#881).
- **Version Drift**: Resolved version inconsistencies between workspace root (0.10.0) and satellite files still referencing 0.9.1 (#884).
- **[HIGH] Path Traversal in DAP Launch**: Fixed path traversal vulnerability in debug adapter (#640).
- **[HIGH] Argument Injection in TestRunner**: Fixed argument injection vulnerability (#633).
- **[MEDIUM] Safe Evaluation Bypass**: Fixed bypass for iterator/IO operations (#647).
- **VS Code Extension Security**: Pinned minimatch to 10.2.3 (#861).
- **Checksum Verification**: Hardened checksum verification and stabilized incremental parsing CI (#858).

### Performance
- **Symbol Extraction**: Optimized regex compilation for faster workspace indexing (#645).
- **Semantic Analyzer**: Eliminated deep cloning of AST nodes in subroutine analysis (#632).
- **Scope Analyzer**: Optimized unused parameter detection (#638).

## [0.9.1] - 2026-02-20

### Added
- **Initial Public Alpha Release**: Substantially complete feature set for early testing.
- **Enhanced LSP Features**: 99% coverage of LSP 3.18 methods (alpha-validated).
- **Complete Semantic Analyzer**: All NodeKind handlers implemented (Phases 1, 2, 3) with 100% AST node coverage.
- **Debug Adapter Protocol (DAP) Support**: Phase 1 bridge to Perl::LanguageServer.
- **Enhanced LSP Cancellation System**: Thread-safe infrastructure for minimal latency.
- **Advanced Code Actions**: AST-aware refactoring including extraction and import optimization.
- **Security Hardening**: UTF-16 boundary fixes and path traversal prevention.
- **Comprehensive API Documentation**: Infrastructure for documentation enforcement.
- **Optimized Test Suite**: 0.31s full test suite execution via adaptive threading.

### Changed
- **Project Origins Documented**: Origins in Q2 2025, forked July 15, 2025 from `tree-sitter-perl-better`.
- **Stability Roadmap Refined**: Formal Stability Contract (contract-locked APIs) pushed to v0.15.0.
- **MSRV Updated**: Minimum Supported Rust Version bumped to 1.92 (Rust 2024 edition).
- **Parser Architecture**: Native recursive descent parser as the primary implementation.

### Fixed
- **v0.9.1 close-out receipts captured**: Workspace index state-machine transitions and early-exit behavior verified.
- **Security boundary fixes**: Resolved multi-root workspace path traversal issues.

## [0.9.0] - 2026-01-18

### Added
- **Semantic Analyzer Phase 1**: 12/12 critical node handlers implemented.
- **LSP textDocument/definition Integration**: Semantic-aware definition resolution.
- **Enhanced Cross-File Navigation**: Dual indexing strategy for improved reference coverage.

### Changed
- **LSP Coverage**: Increased to 82% of trackable features.

## [0.8.8] - 2025-12-01

### Added
- **Initial Workspace Configuration Support**.
- **Enhanced Formatting Fallback**: Always-available capabilities with perltidy integration.

---

## Future Milestones

### Next Release
- Enhanced DAP native implementation (Phase 2).
- Semantic depth improvements for Moo/Moose.

### v0.15.0 (Stability Contract Milestone)
- **Formal Stability Contract**: Contract-locked APIs and wire protocol invariants.
- Full protocol compliance audit.
- Multi-release deprecation cycles.

---

## Version Support Policy (Alpha Phase)

During the alpha phase (pre-v0.15.0):
- **Current Alpha (0.x.y)**: Active development and bug fixes.
- **Breaking Changes**: Allowed in minor (0.x) releases.
- **Security**: Critical patches prioritized for the latest alpha version.
