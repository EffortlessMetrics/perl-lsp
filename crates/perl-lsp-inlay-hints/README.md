# perl-lsp-inlay-hints

Inlay hint generation for Perl LSP.

## Scope

- Produces inlay hints from AST and symbol context.
- Supports parameter hints and lightweight inferred type hints.
- Encodes hint kinds and positions in LSP-friendly structures.

## Public Surface

- `InlayHintsProvider`.
- `InlayHint`, `InlayHintKind`.
- Helper functions: `parameter_hints`, `trivial_type_hints`.

## Workspace Role

Internal feature crate consumed by `perl-lsp` inlay hint requests.

## License

MIT OR Apache-2.0.
