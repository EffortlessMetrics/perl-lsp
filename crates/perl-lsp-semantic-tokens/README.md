# perl-lsp-semantic-tokens

LSP semantic token generation for Perl syntax highlighting.

## Overview

This crate converts Perl source code into LSP semantic tokens by combining lexer-based token classification with AST-driven overlays. Tokens are delta-encoded per the LSP 3.17+ specification for efficient client transmission.

## Public API

| Item | Description |
|------|-------------|
| `collect_semantic_tokens` | Generate delta-encoded tokens from an AST and source text |
| `legend` | Build the `TokensLegend` mapping token type/modifier names to indices |
| `TokensLegend` | Token type and modifier registry with fast lookup map |
| `EncodedToken` | `[u32; 5]` alias: `[deltaLine, deltaStart, length, type, modifiers]` |
| `SemanticTokensProvider` | Placeholder struct for future stateful provider |

## Token Types

namespace, class, function, method, variable, parameter, property, keyword, comment, string, number, regexp, operator, type, macro

## Workspace Role

Internal feature crate in the `tree-sitter-perl-rs` workspace. Consumed by `perl-lsp` to service `textDocument/semanticTokens/full` requests.

## License

MIT OR Apache-2.0
