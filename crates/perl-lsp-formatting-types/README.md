# perl-lsp-formatting-types

Shared DTOs for Perl LSP formatting requests/results:

- `FormattingOptions`
- `FormatPosition`
- `FormatRange`
- `FormatTextEdit`
- `FormattedDocument`

This crate exists to keep transport-safe formatting data structures isolated from
perltidy process execution logic (`perl-lsp-formatting`).
