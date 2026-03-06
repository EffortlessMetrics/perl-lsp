# perl-lsp-document-links

Focused document-link extraction for Perl LSP workflows.

This crate scans source text for `use` and `require` statements and emits
LSP-compatible document links with deferred resolution metadata.
