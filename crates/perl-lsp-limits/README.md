# perl-lsp-limits

Runtime-state microcrate for LSP operation limits and deadline policy.

## Scope

This crate owns:

- Global, thread-safe limit state (`LSP_LIMITS`)
- Fast helper accessors (`workspace_symbol_cap`, `references_cap`, etc.)
- Re-export of the shared `LspLimits` configuration type

The concrete limits configuration shape and profile/update logic live in
`perl-lsp-limits-types` to preserve SRP boundaries.
