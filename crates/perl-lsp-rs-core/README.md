# perl-lsp-rs-core

Implementation core for the perl-lsp-rs LSP server facade.

This crate consolidates the feature and capability-mapping system (Wave F):
- perl-lsp-feature-ids (module: features::ids)
- perl-lsp-feature-contracts (module: features::contracts)
- perl-lsp-feature-flags (module: features::flags)
- perl-lsp-feature-profile (module: features::profile)
- perl-lsp-feature-profile-cli (module: features::profile_cli)
- perl-lsp-feature-policy (module: features::policy)
- perl-lsp-feature-grid (module: features::grid)
- perl-lsp-capability-map (module: capability_map)

The perl-lsp facade re-exports these modules for downstream consumers.

## See Also

- [perl-lsp](../perl-lsp/) — LSP server facade (main public API)
- [.spec/wave-f/](../../.spec/wave-f/) — Wave F specification and implementation checklist
