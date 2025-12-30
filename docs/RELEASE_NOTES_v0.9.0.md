# perl-lsp v0.9.0 - Semantic-Ready Release

## Release Date
December 2025

## Highlights
- **Semantic Analyzer Phase 1 Complete**: 12/12 critical node handlers implemented
- **LSP textDocument/definition Integration**: Semantic-aware definition resolution with proper lexical scoping
- **Revolutionary Performance**: 5000x faster test suite execution with adaptive threading (PR #140)

## What's New

### Semantic Definition Resolution
- SemanticAnalyzer integration with LSP definition handler
- Support for scalars, arrays, hashes, subroutines, and package-qualified calls
- Proper handling of nested scopes, package boundaries, and shadowed variables

### LSP Enhancements
- 91% LSP feature coverage (up from baseline)
- Enhanced cross-file navigation with dual indexing
- Inline completions, inline values, moniker, and linked editing ranges

### Performance Improvements
- LSP behavioral tests: 1560s → 0.31s (5000x improvement)
- Adaptive threading configuration for CI environments
- <1ms incremental parsing updates

## Known Limitations
- Semantic Analyzer Phase 2/3 features deferred (advanced cross-file analysis)
- DAP support at Phase 1 bridge level (v0.1.0)

## Upgrade Notes
No breaking changes from v0.8.8. Seamless upgrade path.
