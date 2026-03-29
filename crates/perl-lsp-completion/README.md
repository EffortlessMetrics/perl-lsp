# perl-lsp-completion

Context-aware completion engine for Perl source.

## When to use this crate

Use `perl-lsp-completion` when you want Perl-aware `textDocument/completion`
behavior without embedding the full language server runtime.

It is the right crate when you need:

- ranked completion results at a source offset
- completion logic that understands Perl sigils, packages, methods, and files
- workspace-aware or AST-aware completion providers in Rust

## Public API

- `CompletionProvider`: builds a symbol table from an AST and optional workspace index, then generates ranked completion items at a given byte offset.
- `CompletionContext`: request-scoped context for trigger character, scope, and prefix handling.
- `CompletionItem` and `CompletionItemKind`: completion payloads with insert text, sort priority, and text-edit range.

## Example

```rust,ignore
use perl_lsp_completion::CompletionProvider;

let provider = CompletionProvider::new(&ast, Some(&workspace_index))?;
let completions = provider.get_completions(source, position)?;
assert!(!completions.is_empty());
```

## Workspace role

Internal feature crate consumed by `perl-lsp` for completion handling. It is
mostly a workspace building block rather than a standalone end-user crate.

## License

MIT OR Apache-2.0
