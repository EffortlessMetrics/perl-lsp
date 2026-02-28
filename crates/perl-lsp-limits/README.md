# perl-lsp-limits

Standalone microcrate for LSP operation limits and deadline policy.

## Scope

This crate owns:

- Result caps (`workspace_symbol_cap`, `references_cap`, etc.)
- Deadlines for bounded request latency
- Cache and indexing guardrails
- Global, thread-safe limit state and helper accessors

Keeping these concerns in one place supports SRP and allows other crates to share the same bounded behavior contract.
