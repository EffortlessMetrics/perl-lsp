# perl-lsp-completion

Context-aware LSP completion engine for Perl, providing intelligent suggestions
for variables, functions, methods, packages, keywords, and file paths.

## Public API

- `CompletionProvider` -- builds a symbol table from an AST and optional workspace index, then generates ranked completion items at a given byte offset.
- `CompletionContext` -- request-scoped context (position, trigger character, scope, prefix).
- `CompletionItem` / `CompletionItemKind` -- completion payloads with label, insert text, sort priority, and text edit range.

## Completion Sources

| Source | Trigger | Module |
|--------|---------|--------|
| Variables | `$`, `@`, `%` sigils | `variables` |
| Functions | identifier prefix | `functions` |
| Built-ins | identifier prefix | `builtins` |
| Keywords | identifier prefix (with snippets) | `keywords` |
| Methods | `->` | `methods` (includes DBI inference) |
| Packages | `::` | `packages` (workspace index) |
| Workspace symbols | identifier prefix | `workspace` |
| Test::More | test file context | `test_more` |
| File paths | inside string literals | `file_path` (secure traversal) |
| Moo/Moose `has` options | inside `has(...)` | built-in |

## Workspace Role

Internal feature crate consumed by `perl-lsp` for `textDocument/completion` handling. Not intended for direct end-user use outside the workspace.

## License

MIT OR Apache-2.0
