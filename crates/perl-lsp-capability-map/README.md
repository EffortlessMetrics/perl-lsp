# perl-lsp-capability-map

Standalone SRP microcrate for capability/feature translation in the Perl LSP ecosystem.

## Responsibility

- Convert `lsp_types::ServerCapabilities` into canonical Perl LSP feature IDs.
- Build `ServerCapabilities` from feature ID lists.

This keeps capability mapping concerns separate from feature contracts/catalog metadata.
