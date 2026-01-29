# CLAUDE.md

This file provides guidance to Claude Code when working with code in this repository.

## Crate Overview

`perl-lsp-semantic-tokens` is a **Tier 4 LSP feature crate** providing semantic syntax highlighting.

**Purpose**: LSP semantic tokens provider for Perl — enables rich syntax highlighting based on semantic analysis.

**Version**: 0.8.8

## Commands

```bash
cargo build -p perl-lsp-semantic-tokens      # Build this crate
cargo test -p perl-lsp-semantic-tokens       # Run tests
cargo clippy -p perl-lsp-semantic-tokens     # Lint
cargo doc -p perl-lsp-semantic-tokens --open # View documentation
```

## Architecture

### Dependencies

- `perl-parser-core` - AST access
- `perl-lexer` - Token information
- `perl-semantic-analyzer` - Semantic classification
- `lsp-types` - LSP semantic token types

### LSP Capabilities

| Method | Purpose |
|--------|---------|
| `textDocument/semanticTokens/full` | All tokens |
| `textDocument/semanticTokens/delta` | Changed tokens only |
| `textDocument/semanticTokens/range` | Tokens in range |

### Token Types

| Type | Usage |
|------|-------|
| `namespace` | Package names |
| `type` | Type names |
| `class` | Class names |
| `parameter` | Subroutine parameters |
| `variable` | Variables |
| `property` | Hash keys, object properties |
| `function` | Subroutines |
| `method` | Methods |
| `keyword` | Keywords |
| `modifier` | Modifiers (my, our, etc.) |
| `comment` | Comments |
| `string` | Strings |
| `number` | Numbers |
| `regexp` | Regex patterns |
| `operator` | Operators |

### Token Modifiers

| Modifier | Usage |
|----------|-------|
| `declaration` | Variable/sub declaration |
| `definition` | Definition site |
| `readonly` | Constants, readonly vars |
| `static` | Package variables |
| `deprecated` | Deprecated symbols |
| `modification` | Write access |

## Usage

```rust
use perl_lsp_semantic_tokens::SemanticTokensProvider;

let provider = SemanticTokensProvider::new(analyzer);

// Get all semantic tokens
let tokens = provider.full(document)?;

// Get delta from previous
let delta = provider.delta(document, previous_result_id)?;
```

### Token Encoding

Tokens are encoded as relative positions:

```rust
// [deltaLine, deltaStartChar, length, tokenType, tokenModifiers]
// Each token is relative to previous token
[0, 0, 3, 7, 0],   // "sub" at line 0, col 0
[0, 4, 8, 11, 1],  // "my_func" at line 0, col 4 (declaration)
```

## Important Notes

- Semantic tokens enhance TextMate grammar highlighting
- Delta requests improve performance on edits
- Token types/modifiers are negotiated during initialize
- Classification uses semantic analysis for accuracy
