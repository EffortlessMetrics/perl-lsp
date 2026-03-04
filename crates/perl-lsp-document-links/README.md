# perl-lsp-document-links

Focused document-link extraction for Perl source files used by LSP providers.

This crate parses `use`/`require` heads and emits deferred document-link payloads
for `textDocument/documentLink` and `documentLink/resolve` workflows.
