# API Stability & SemVer - v0.8.8+ GA

**MSRV:** 1.89 • **Edition:** 2024 • **Status:** General Availability

## Published Crates (v0.8.8+ GA)

| Crate | Version | Purpose | Stability |
|-------|---------|---------|-----------|
| [perl-parser](https://crates.io/crates/perl-parser) | 0.8.8+ | Parser & LSP | Stable (GA) |
| [perl-lexer](https://crates.io/crates/perl-lexer) | 0.8.8+ | Tokenizer | Stable (GA) |
| [perl-corpus](https://crates.io/crates/perl-corpus) | 0.8.8+ | Test corpus | Stable |
| [perl-parser-pest](https://crates.io/crates/perl-parser-pest) | 0.8.8+ | Legacy parser | Deprecated |

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
- **Feature set**: ~35% functional (see [LSP_ACTUAL_STATUS.md](../LSP_ACTUAL_STATUS.md))
- **Working features**: Diagnostics, completion, go-to-definition (single file)
- **Stub features**: Many return empty results (workspace ops, refactoring)

## Additive guarantee
- We **add** `NodeKind` variants and token types in **minor** releases
- We **do not rename or remove** existing variants until 1.0
- `to_sexp()` output is stable for test automation (modulo whitespace/additional metadata)
- New LSP capabilities are added without breaking existing clients

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

## Performance guarantees
- Simple files (~100 lines): <10µs parsing time
- Medium files (~1000 lines): <200µs parsing time  
- Large files (~10K lines): <2ms parsing time
- LSP response time: <50ms for all operations
- Memory usage: O(n) with input size, no exponential blowups

## Compatibility matrix

| perl-parser | perl-lexer | perl-corpus | Notes |
|-------------|------------|-------------|-------|
| 0.8.8+ | 0.8.8+ | 0.8.8+ | Current GA release |
| 0.8.x | 0.8.x | 0.8.x | Patch versions compatible |
| 0.9.x | 0.9.x | 0.9.x | Next minor (breaking allowed) |

## Migration guides
- **From 0.7.x to 0.8.x**: See CHANGELOG.md for position helper changes
- **From tree-sitter-perl C**: Use migration guide in docs/MIGRATION.md
- **LSP client authors**: Follow LSP_ACTUAL_STATUS.md for capability negotiation

## Support lifecycle
- **Current stable**: 0.8.x (security fixes + critical bugs)
- **Previous stable**: 0.7.x (security fixes only until 2025-04-01)
- **LTS planning**: First LTS will be 1.0.0 (2026 target)

## How to report stability issues
1. Check this document first for guarantees
2. File issue with minimal reproduction
3. Tag with `api-stability` label
4. Include version numbers and feature flags used

---

*This document is authoritative for API stability questions. Last updated: 2025-09-01 (v0.8.8+ GA)*