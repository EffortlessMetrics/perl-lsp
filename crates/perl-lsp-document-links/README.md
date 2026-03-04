# perl-lsp-document-links

Focused SRP microcrate for extracting Perl `use`/`require` document links for LSP responses.

## API

- `compute_links(uri, text, roots)` - scans source text and returns deferred-resolve link payloads.
