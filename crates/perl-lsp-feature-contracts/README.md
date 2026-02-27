# perl-lsp-feature-contracts

Shared, microcrate-level contracts for LSP feature flags and capability
translation used by the server runtime and external tooling.

This crate is intentionally small and stable:

- Canonical feature metadata from `features.toml`
- Canonical IDs for advertised capability contracts
- BDD-friendly feature rows for coverage and reporting
- SRP-focused bidirectional server capability mapping
- Coverage-aware percent helpers for grid-style reporting
