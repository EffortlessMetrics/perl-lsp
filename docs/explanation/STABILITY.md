# API Stability & SemVer - v0.8.8 GA

**MSRV:** 1.89 • **Edition:** 2024 • **Status:** Production Ready with Enhanced Workspace Configuration

## Published Crates (v0.8.8 GA)

| Crate | Version | Purpose | Stability |
|-------|---------|---------|-----------|
| [perl-parser](https://crates.io/crates/perl-parser) | 0.8.9 | Parser & LSP | Stable (production-ready) |
| [perl-lexer](https://crates.io/crates/perl-lexer) | 0.8.9 | Tokenizer | Stable (production-ready) |
| [perl-corpus](https://crates.io/crates/perl-corpus) | 0.8.9 | Test corpus | Stable |
| [perl-parser-pest](https://crates.io/crates/perl-parser-pest) | 0.8.9 | Legacy parser | Deprecated |
| [perl-lsp](https://crates.io/crates/perl-lsp) | 0.8.9 | LSP Server Binary | Stable (dedicated crate) |

## What's considered stable (≥0.8.x)

### perl-parser
- **Core API**: `Parser`, `parse(&str) -> Result<Node, ParseError>`, `Node`, `NodeKind`, `SourceLocation`
- **Serialization**: `to_sexp()` - Tree-sitter compatible S-expression output
- **Position helpers**: UTF-8 byte offset ↔ UTF-16 position conversion utilities
- **AST traversal**: Read-only visitor pattern for tree walking

### perl-lexer
- **Tokenization**: `PerlLexer`, `Token`, `TokenType` (variants may be added)
- **Mode-aware lexing**: Context-sensitive tokenization with lookahead

### perl-corpus
- **Test generation**: Public generator functions (signatures stable; output is best-effort)
- **Fuzzing support**: Property-based test generators with deterministic seeds

### perl-lsp (binary)
- **LSP interface**: `--stdio` mode with standard LSP request/response protocol
- **Feature set**: ~85% functional (see [LSP_ACTUAL_STATUS.md](../LSP_ACTUAL_STATUS.md))
- **Working features**: Enhanced diagnostics, completion, go-to-definition, workspace symbols, bless parsing, symbol extraction
- **Enhanced v0.8.8**: Production-ready workspace configuration with reliable build system
- **Backward compatible**: All existing features maintained while adding workspace reliability

## Additive guarantee (Enhanced v0.8.8)
- We **add** `NodeKind` variants and token types in **minor** releases
- We **do not rename or remove** existing variants until 1.0
- `to_sexp()` output is stable for test automation (modulo whitespace/additional metadata)
- **Enhanced v0.8.8**: Production-ready workspace configuration ensures reliable builds across platforms
- **Enhanced v0.8.8**: LSP crate separation provides cleaner architecture without breaking existing APIs
- New LSP capabilities are added without breaking existing clients
- **Backward compatibility**: All v0.8.7 and earlier APIs remain fully functional in v0.8.8

## Workspace Configuration Stability (v0.8.8+)
- **Build reliability**: Workspace excludes system-dependent crates (tree-sitter-perl-c, etc.)
- **Platform independence**: Clean builds on all platforms without libclang or C dependencies
- **Test stability**: 291+ tests pass consistently with zero flaky failures
- **CI stability**: Resolved timeout issues and feature conflicts for reliable automation
- **Production focus**: Testing only published crate APIs ensures user-facing stability

See [WORKSPACE_TEST_REPORT.md](../WORKSPACE_TEST_REPORT.md) for detailed workspace configuration status.

## Breaking changes policy (pre-1.0)
- Breaking changes only in **minor** (0.Y.Z) releases with clear CHANGELOG notes
- MSRV bumps occur only in minor releases and are prominently documented
- Deprecated items receive at least one minor release warning before removal

## Error model
- `parse()` never panics on malformed input: returns `ParseError` with location
- LSP server never panics; unknown requests return structured error responses
- All errors include source location for debugging
- Recovery mechanisms attempt to continue parsing after errors

## Position encoding
- **Parser positions**: UTF-8 byte offsets (0-based)
- **LSP positions**: UTF-16 code unit offsets with 0-based lines/columns
- **Stable converters**: `offset_to_position()`, `position_to_offset()`
- **Line endings**: CRLF and LF both supported transparently

## Feature flags

| Flag | Purpose | Stability |
|------|---------|-----------|
| `pure-rust` | Enable Pest-based parser (v2) | Stable |
| `ts-compat` | Tree-sitter compatibility mode | Stable |
| `cli` | Build command-line binaries | Stable |
| `workspace` | Cross-file analysis (experimental) | Unstable |
| `expose_lsp_test_api` | Test-only LSP internals | Test only |

## Performance guarantees (v0.8.8 Confirmed Metrics)
- Simple files (~100 lines): <10µs parsing time (actual: 6-8µs)
- Medium files (~1000 lines): <200µs parsing time (actual: 12-18µs)
- Large files (~10K lines): <2ms parsing time (actual: 150-200µs)
- LSP response time: <50ms for all operations (actual: <20ms typical)
- Memory usage: O(n) with input size, no exponential blowups
- **Performance improvement**: 4-19x faster than legacy implementations with maintained API stability

## Compatibility matrix

| perl-parser | perl-lexer | perl-corpus | perl-lsp | Notes |
|-------------|------------|-------------|----------|-------|
| 0.8.9 | 0.8.9 | 0.8.9 | 0.8.9 | Current GA (production workspace config) |
| 0.8.8 | 0.8.8 | 0.8.8 | - | Previous GA (enhanced reliability) |
| 0.8.x | 0.8.x | 0.8.x | 0.8.x | All patch versions compatible |
| 0.8.7+ | 0.8.7+ | 0.8.7+ | - | Enhanced features, backward compatible |
| 0.9.x | 0.9.x | 0.9.x | 0.9.x | Next minor (breaking allowed) |

## Migration guides
- **From 0.7.x to 0.8.x**: See CHANGELOG.md for position helper changes
- **From tree-sitter-perl C**: Use migration guide in docs/archive/MIGRATION.md
- **LSP client authors**: Follow LSP_ACTUAL_STATUS.md for capability negotiation

## Support lifecycle
- **Current stable**: 0.8.9 (security fixes + critical bugs + production workspace reliability)
- **Previous stable**: 0.8.8 (backward compatible, enhanced reliability, maintenance mode)
- **Previous stable**: 0.8.7 (critical fixes only until 2025-06-01)
- **Previous stable**: 0.7.x (security fixes only until 2025-04-01)
- **LTS planning**: First LTS will be 1.0.0 (2026 target)

## How to report stability issues
1. Check this document first for guarantees
2. File issue with minimal reproduction
3. Tag with `api-stability` label
4. Include version numbers and feature flags used

---

*This document is authoritative for API stability questions. Last updated: 2025-09-05 (v0.8.8 GA with Production Workspace Configuration)*