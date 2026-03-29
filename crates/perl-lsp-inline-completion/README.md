# perl-lsp-inline-completion

Deterministic inline completion for Perl LSP clients.

## Problem it solves

Inline completions need fast, predictable suggestions that can be shown as
ghost text while the user types. This crate prepares local editing context and
returns deterministic completions based on syntax and nearby code, without any
AI dependency.

## Public API

- `InlineCompletionProvider` prepares context and returns suggestions.
- `PreparedInlineCompletionContext` captures the visible local context.
- `InlineCompletionItem` and `InlineCompletionList` mirror the LSP preview
  payload shape.

## Example

```rust,ignore
use perl_lsp_inline_completion::InlineCompletionProvider;

let provider = InlineCompletionProvider::new();
let completions = provider.get_inline_completions(source, line, character);
```

## What it suggests

- `new()` after `->`
- common pragmas after `use `
- body scaffolds for new subroutines
- nearby variables and imports when the local context supports them

## Workspace role

`perl-lsp` uses this crate to implement preview inline completion behavior while
keeping the ranking and context logic isolated and testable.

## License

MIT OR Apache-2.0
