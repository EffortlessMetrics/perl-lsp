# Issue #145: Critical LSP features have ignored tests - executeCommand and code actions missing

## Context

The Perl LSP server has several critical LSP features that are either completely missing or have ignored tests, significantly limiting IDE functionality and developer productivity. The main gaps identified are:

1. **Missing executeCommand Implementation**: The `perl.runCritic` command is partially implemented but not properly wired to the LSP protocol
2. **Incomplete Code Action Integration**: Advanced refactoring operations exist but are not accessible through the LSP `textDocument/codeAction` handler
3. **Ignored Test Infrastructure**: 9+ code action refactoring tests and perlcritic integration tests are ignored, indicating incomplete implementation

**Affected Perl LSP Components**:
- **perl-parser**: Core LSP provider implementations need executeCommand wiring
- **perl-lsp**: LSP server binary requires protocol method handlers
- **perl-corpus**: Test corpus validation for comprehensive coverage

**LSP Workflow Impact**: This affects the complete Parse → Index → Navigate → Complete → Analyze → **Execute** workflow, blocking advanced IDE features like integrated code quality analysis and sophisticated refactoring operations.

**Performance Implications**: Implementation must maintain <50ms code action response times and <2s executeCommand execution while preserving incremental parsing efficiency.

## User Story

As a Perl developer using LSP-compatible editors (VSCode, Neovim, Emacs), I want comprehensive executeCommand and code action refactoring support, so that I can access advanced IDE features including integrated perlcritic analysis and sophisticated refactoring capabilities that are critical for productive enterprise Perl development.

## Acceptance Criteria

AC1: Complete executeCommand LSP method implementation with `perl.runCritic` command support
- Implement `workspace/executeCommand` handler in LSP server with protocol compliance
- Support `perl.runCritic` command with dual analyzer strategy (external perlcritic + built-in fallback)
- Advertise executeCommand capabilities in server initialization response
- Handle unknown commands with structured error responses and graceful degradation
- Return structured results with status, violations array, analyzer used, and execution time

AC2: Enable and validate perlcritic executeCommand integration tests
- Remove `#[ignore]` attribute from perlcritic executeCommand test in behavioral test suite
- Validate external perlcritic execution vs built-in analyzer fallback scenarios
- Test structured violation output with LSP diagnostic integration workflow
- Verify performance requirements: <2s execution time for typical Perl files (<500 lines)
- Test graceful degradation when perlcritic tool unavailable

AC3: Wire existing code action implementations to LSP protocol handlers
- Integrate `EnhancedCodeActionsProvider` with `textDocument/codeAction` LSP method
- Support RefactorExtract (extract variable/subroutine), RefactorRewrite (code quality), and SourceOrganizeImports actions
- Implement code action resolve capability for complex refactorings with preview support
- Maintain <50ms response time for code action suggestions with incremental analysis
- Ensure cross-file refactoring uses dual indexing pattern for 98% reference coverage

AC4: Comprehensive import management code actions
- Wire existing `ImportOptimizer` to SourceOrganizeImports code action category
- Implement remove unused imports, add missing imports, alphabetical sorting with categorization
- Support workspace-aware dependency analysis with Package::module resolution
- Test import optimization across multiple files with proper AST integration
- Validate performance: <100ms for import analysis on typical Perl modules

AC5: Integration test suite for executeCommand and code actions features
- Add comprehensive end-to-end tests for executeCommand + code action workflows
- Test LSP protocol compliance with executeCommand capabilities advertisement
- Validate error handling scenarios: missing files, syntax errors, tool unavailability
- Performance regression testing with benchmark baselines for response times
- Cross-file refactoring validation with workspace setup and dual indexing verification

## Technical Implementation Notes

- **Affected crates**: perl-parser (LSP provider logic), perl-lsp (protocol handlers), perl-corpus (test validation)
- **LSP workflow stages**: All stages affected, particularly Execute and Analyze phases with integrated quality feedback
- **Performance considerations**: Maintain <50ms code action responses, <2s executeCommand execution, preserve <1ms incremental parsing
- **Parsing requirements**: Leverage existing ~100% Perl syntax coverage with enhanced AST integration for refactoring analysis
- **Cross-file navigation**: Utilize established dual indexing strategy for Package::subroutine resolution in extract operations
- **Protocol compliance**: LSP 3.17+ specification adherence with executeCommand and codeAction method implementations
- **Tree-sitter integration**: Code action analysis compatible with existing highlight testing infrastructure
- **Testing strategy**: TDD with `// AC:ID` tags, comprehensive LSP protocol compliance testing, adaptive threading support for CI environments with `RUST_TEST_THREADS=2`
- **Enterprise security**: Maintain existing path traversal prevention and file completion safeguards in refactoring operations
- **Thread safety**: Ensure executeCommand operations are thread-safe with existing adaptive threading configuration
- **Error handling**: Implement proper `anyhow::Result<T>` patterns with LSP-compliant error responses and structured diagnostic integration