# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased] - v0.9.0-semantic-lsp-ready

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

- **Test Infrastructure Improvements**
  - **Semantic Unit Tests**: Direct validation of `SemanticAnalyzer` core without LSP overhead
    - `test_analyzer_find_definition_scalar`: Direct analyzer testing
    - `test_semantic_model_definition_at`: SemanticModel API validation
  - **Dynamic Test Positions**: All tests calculate positions from code strings, eliminating brittleness
  - **Resource-Constrained Execution**: Commands optimized for limited CPU/RAM environments
  - **Clear Test Documentation**: Comprehensive command reference in CLAUDE.md and docs

### Changed - Project Status & Documentation

- **Core Goal Progress**: Updated to ~80-85% "fully working" for "Perl parser + LSP that actually works"
- **Parser & Heredocs**: Marked as ~95-100% complete - functionally done for v1.0
- **Sprint A**: Marked as ✅ **100% COMPLETE** (parser foundation + heredocs/statement tracker)
- **Sprint B Phase 1**: Semantic analyzer core ✅ complete, ready for Phase 2/3 advanced features
- **MVP Completion**: Updated from 70-75% to 75-80% (parser done, semantics Phase 1 done)
- **Issue #182 (Statement Tracker)**: Ready to close - all implementation and testing complete
- **ROADMAP.md**: Added "Validation & De-Risking" phase with 3-band path to v1.0
- **Status Documentation**: Comprehensive updates to reflect semantic analyzer completion

### Documentation

- **CLAUDE.md**: Added semantic definition testing commands and updated status metrics
- **ROADMAP.md**: New "Current Phase: Validation & De-Risking" section with concrete path forward
- **Path to v1.0**: Documented 3-band approach (prove semantic stack, reduce ignored tests, tag v0.9)
- **Known Constraints**: Resource limits, CI billing, ignored test count (779 tests)
- **Test Commands**: Resource-efficient semantic testing commands for constrained environments

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
