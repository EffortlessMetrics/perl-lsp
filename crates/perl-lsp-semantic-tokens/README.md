# perl-lsp-semantic-tokens

Semantic token encoding for Perl LSP syntax highlighting.

## Scope

- Collects semantic tokens from parser/semantic data.
- Encodes tokens using LSP semantic token wire format.
- Provides token legend metadata used by editor clients.

## Public Surface

- `collect_semantic_tokens`.
- `legend`, `TokensLegend`.
- `EncodedToken`.
- `SemanticTokensProvider` (thin wrapper API).

## Workspace Role

Internal feature crate consumed by `perl-lsp` semantic token requests.

## License

MIT OR Apache-2.0.
