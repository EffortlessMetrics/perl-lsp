# perl-lsp-document-highlight

Symbol occurrence highlighting for Perl source files.

## Problem it solves

When the cursor lands on a variable, function, or method, editors expect all
matching occurrences in the current document to light up immediately. This
crate finds those occurrences and classifies them as read or write access where
possible.

## Public API

- `DocumentHighlightProvider` performs the highlight lookup.
- `DocumentHighlight` describes each highlighted range.
- `DocumentHighlightKind` distinguishes text, read, and write occurrences.

## Example

```rust,ignore
use perl_lsp_document_highlight::DocumentHighlightProvider;

let provider = DocumentHighlightProvider::new();
let highlights = provider.find_highlights(&ast, source, byte_offset);
```

## Workspace role

`perl-lsp` uses this crate to implement `textDocument/documentHighlight`
without duplicating symbol-matching logic in the main server crate.

## License

MIT OR Apache-2.0
