# perl-lsp-completion

Perl-aware completion engine for `textDocument/completion`. It turns an AST,
the current cursor location, and optional workspace context into ranked
completion items for variables, functions, keywords, packages, methods, file
paths, and test helpers.

## Use this crate when

Use `perl-lsp-completion` if you need the completion logic itself. Use
`perl-lsp-completion-item` if you only need the item types and deterministic
sorting, and use `perl-lsp-providers` when you want the umbrella re-export
surface for the whole provider stack.

## Key exports

- `CompletionProvider` - builds completion results from AST and optional index
- `CompletionContext` - request-scoped state such as prefix and trigger
- `CompletionItem` / `CompletionItemKind` - normalized completion payloads
- `get_dbi_method_documentation` and `get_test_more_documentation` - Perl-aware
  documentation helpers for completion hints

## Example

```rust,ignore
use perl_lsp_completion::CompletionProvider;

let provider = CompletionProvider::new_with_index(&ast, None);
let items = provider.get_completions(source, byte_offset);
```

## Stack role

This is the feature crate that backs `perl-lsp` completion handling. It sits
above parsing and workspace indexing, and below the editor-facing LSP runtime.
