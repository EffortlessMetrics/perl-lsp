# perl-lsp-document-highlight

Perl document-highlight provider for symbol-occurrence highlighting around the
cursor. It answers the question "where else is this thing used in the current
document?"

## Use this crate when

Use `perl-lsp-document-highlight` if you need the highlight logic itself. It is
lighter-weight than the navigation crates and narrower than the full provider
umbrella.

## Key exports

- `DocumentHighlightProvider` - finds highlights for the active symbol
- `DocumentHighlight` / `DocumentHighlightKind` - serialized highlight payloads

## Example

```rust,ignore
use perl_lsp_document_highlight::DocumentHighlightProvider;

let provider = DocumentHighlightProvider::new();
let highlights = provider.find_highlights(&ast, line, character, source);
```

## Stack role

`perl-lsp` uses this crate for `textDocument/documentHighlight`. It sits on top
of parsed source and symbol lookup.
