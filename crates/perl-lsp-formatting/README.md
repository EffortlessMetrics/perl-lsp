# perl-lsp-formatting

Document and range formatting support for Perl LSP.

## Scope

- Wraps `perltidy` execution via `perl-lsp-tooling` runtime abstractions.
- Produces LSP-compatible text edits for full-document and range formatting.
- Handles formatting options and error reporting in a testable interface.

## Public Surface

- `FormattingProvider`.
- `FormattingOptions`, `FormattingError`.
- Edit/result types: `FormatTextEdit`, `FormatRange`, `FormatPosition`, `FormattedDocument`.

## Workspace Role

Internal feature crate consumed by `perl-lsp` formatting requests.

## License

MIT OR Apache-2.0.
