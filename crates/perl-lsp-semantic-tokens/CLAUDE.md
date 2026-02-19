# CLAUDE.md

This file provides guidance to Claude Code when working with code in this repository.

## Crate Overview

`perl-lsp-semantic-tokens` is a **Tier 2 LSP feature crate** providing semantic token generation for Perl syntax highlighting.

**Purpose**: Combines lexer token classification with AST overlays to produce delta-encoded semantic tokens per the LSP 3.17+ specification.

**Version**: 0.9.0

## Commands

```bash
cargo build -p perl-lsp-semantic-tokens      # Build this crate
cargo test -p perl-lsp-semantic-tokens       # Run tests
cargo clippy -p perl-lsp-semantic-tokens     # Lint
cargo doc -p perl-lsp-semantic-tokens --open # View documentation
```

## Architecture

### Dependencies

- `perl-parser-core` — AST node types (`Node`, `NodeKind`)
- `perl-lexer` — `PerlLexer` and `TokenType` for fast-path token classification
- `perl-semantic-analyzer` — `get_node_children` helper for AST walking
- `rustc-hash` — `FxHashMap` for token type index lookup
- `lsp-types`, `serde`, `thiserror` — LSP types, serialization, error handling

### Key Types and Modules

| Item | Location | Description |
|------|----------|-------------|
| `EncodedToken` | `semantic_tokens.rs` | `[u32; 5]` delta-encoded token format |
| `TokensLegend` | `semantic_tokens.rs` | Token type/modifier registry with `FxHashMap` lookup |
| `legend()` | `semantic_tokens.rs` | Builds the standard 15-type, 7-modifier legend |
| `collect_semantic_tokens()` | `semantic_tokens.rs` | Main entry point: lexer pass + AST overlay + dedup + encode |
| `SemanticTokensProvider` | `lib.rs` | Placeholder struct (unit struct, `Default` impl) |

### Token Generation Pipeline

1. **Lexer pass** — `PerlLexer` classifies keywords, strings, numbers, regexps, operators, comments
2. **AST overlay** — `walk_ast` adds package, subroutine, function call, method call, variable nodes
3. **Dedup** — `remove_overlapping_tokens` resolves conflicts (longer token wins on same line)
4. **Encode** — `encode_raw_tokens_to_deltas` produces relative-position `[u32; 5]` arrays

### Token Types (15)

namespace, class, function, method, variable, parameter, property, keyword, comment, string, number, regexp, operator, type, macro

### Token Modifiers (7)

declaration, definition, readonly, defaultLibrary, deprecated, static, async

## Usage

```rust
use perl_lsp_semantic_tokens::{collect_semantic_tokens, legend, EncodedToken};

let leg = legend();
let tokens: Vec<EncodedToken> = collect_semantic_tokens(&ast, source, &to_pos16);
```

## Important Notes

- Tokens are single-line only; multi-line spans emit `len = 0` and are skipped
- Overlap resolution prefers longer tokens; equal-length keeps the first encountered
- The `SemanticTokensProvider` struct is a placeholder; all logic is in `collect_semantic_tokens`
- Tests include mutation-hardening cases for overlap detection edge cases (Issue #155)
- No `unwrap()`/`expect()` in production code per workspace lint policy
