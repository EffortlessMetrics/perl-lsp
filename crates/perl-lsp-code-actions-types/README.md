# perl-lsp-code-actions-types

Shared DTOs and enums for Perl LSP code action pipelines:

- `QuickFixDiagnostic`
- `CodeAction`
- `CodeActionKind`
- `CodeActionEdit`

This crate exists to isolate transport-safe code action data structures from
provider and rule execution logic (`perl-lsp-code-actions`).
