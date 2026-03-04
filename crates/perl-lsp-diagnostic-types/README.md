# perl-lsp-diagnostic-types

Shared diagnostic model types used across Perl LSP crates.

This crate contains only the data model (`Diagnostic`, severity, tags, and related information),
allowing crates that only need these shared structures to avoid depending on the full diagnostics engine.
