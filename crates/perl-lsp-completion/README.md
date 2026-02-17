# perl-lsp-completion

Context-aware completion engine for Perl LSP.

## Scope

- Generates completion items from parser/semantic/workspace data.
- Supports builtin, variable, function, package, and file-path completion paths.
- Provides ranking/sorting tuned for editor completion UX.

## Public Surface

- `CompletionProvider`.
- `CompletionContext`.
- `CompletionItem`, `CompletionItemKind`.

## Workspace Role

Internal feature crate consumed by `perl-lsp` `textDocument/completion` handling.

## License

MIT OR Apache-2.0.
