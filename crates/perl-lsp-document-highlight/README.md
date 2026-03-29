# perl-lsp-document-highlight

[![Crates.io](https://img.shields.io/crates/v/perl-lsp-document-highlight.svg)](https://crates.io/crates/perl-lsp-document-highlight)
[![Documentation](https://docs.rs/perl-lsp-document-highlight/badge.svg)](https://docs.rs/perl-lsp-document-highlight)

Symbol-occurrence highlighting for Perl editors and language servers.

## When to use this crate

Use `perl-lsp-document-highlight` when you need
`textDocument/documentHighlight` behavior for Perl source. Given a parsed AST,
source text, and cursor offset, it returns the matching symbol occurrences in
the current file and distinguishes read vs write access where possible.

This crate is intended for Rust-based LSP integrations, especially the
`perl-lsp` workspace.

## Quick example

```rust,ignore
use perl_lsp_document_highlight::DocumentHighlightProvider;

let provider = DocumentHighlightProvider::new();
let highlights = provider.find_highlights(&ast, source, cursor_byte_offset);
assert!(!highlights.is_empty());
```

## Public API

- `DocumentHighlightProvider`: main entry point
- `DocumentHighlight`: returned range plus highlight kind
- `DocumentHighlightKind`: `Text`, `Read`, or `Write`

## License

MIT OR Apache-2.0
