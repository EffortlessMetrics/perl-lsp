# perl-lsp-document-links

Focused document link extraction utilities for Perl source files used by LSP implementations.

## API

- `compute_links(uri, text, roots)` scans `use`/`require` statements and emits deferred link metadata for `documentLink/resolve`.
