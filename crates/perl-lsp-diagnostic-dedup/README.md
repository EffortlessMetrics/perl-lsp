# perl-lsp-diagnostic-dedup

Diagnostic de-duplication helpers shared across Perl LSP crates.

This crate provides utilities for sorting and removing duplicate diagnostics while
keeping a small dependency surface by depending only on `perl-lsp-diagnostic-types`.
