# perl-lsp-inline-completion

Perl inline-completion provider for ghost-text style suggestions. It prepares
request context from the current buffer and returns inline completions for the
editor to render.

## Use this crate when

Use `perl-lsp-inline-completion` if you need inline suggestions, not the normal
completion-list UI. It is adjacent to `perl-lsp-completion`, but optimized for
single-line, in-buffer suggestions.

## Key exports

- `InlineCompletionProvider` - computes inline completions from buffer state
- `PreparedInlineCompletionContext` - parsed context for a cursor location
- `InlineCompletionItem` / `InlineCompletionList` - inline completion payloads

## Example

```rust,ignore
use perl_lsp_inline_completion::InlineCompletionProvider;

let provider = InlineCompletionProvider::new();
let completions = provider.get_inline_completions("sub hello", 0, 9);
```

## Stack role

`perl-lsp` uses this crate for inline completion requests. It sits on top of
the same Perl-aware parsing and context analysis as the standard completion
engine.
