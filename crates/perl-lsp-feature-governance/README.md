# perl-lsp-feature-governance

Facade crate for Perl LSP feature governance.

This crate composes focused microcrates so consumers can use one stable API for:

- Feature profile parsing (`ga-lock`, `production`, `all`, aliases)
- Feature-flag policy resolution
- BDD grid JSON/report payloads
- Capability/feature-id interoperability

Use this crate when you want the full governance surface without importing each
lower-level feature crate directly.
