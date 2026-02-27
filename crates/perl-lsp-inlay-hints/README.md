# perl-lsp-inlay-hints

LSP inlay hints provider for Perl. Generates parameter name hints for built-in
function calls and lightweight type hints for literals, producing
LSP-compatible `InlayHint` values from a parsed AST.

## Public API

- **`InlayHintsProvider`** -- unit struct with `generate_hints()`, `parameter_hints()`, and `trivial_type_hints()` methods. Implements `Default`.
- **`InlayHint`** / **`InlayHintKind`** -- hint data types (position, label, kind, padding).
- **`parameter_hints()`** / **`trivial_type_hints()`** -- free functions returning `serde_json::Value` vectors.

## Hint Categories

- **Parameter hints** -- labels arguments to 14 built-in Perl functions (`open`, `split`, `substr`, `push`, `map`, `grep`, `sort`, `join`, `sprintf`, `printf`, `index`, `rindex`, `splice`, `pack`/`unpack`).
- **Type hints** -- annotates literals with inferred types: `Num`, `Str`, `Hash`, `Array`, `Regex`, `CodeRef`.

## Workspace Role

Internal feature crate in the `tree-sitter-perl-rs` workspace. Consumed by `perl-lsp` to handle `textDocument/inlayHint` requests. Depends on `perl-parser-core` (AST), `perl-position-tracking` (positions), and `perl-semantic-analyzer` (AST traversal).

## License

MIT OR Apache-2.0
