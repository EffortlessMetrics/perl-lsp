# perl-feature-catalog

Small, crates.io-friendly crate that owns feature-catalog parsing, validation, and code-generation helpers.

## Purpose

- Keep feature source-of-truth in `features.toml` consistent across crates.
- Centralize catalog loading, validation, and source selection (`features.toml` vs override).
- Shared code-generation for LSP/DAP generated feature modules.
- Stable APIs for BDD-style verification and reporting workflows in `xtask`.

## Public APIs

- `Catalog`, `Feature`, `Meta`, `Maturity`, `AreaStats`, `CatalogError`
- `resolve_catalog_source`, `load_catalog_for_build`, `read_catalog`
- `Catalog::validate`, `Catalog::advertised_feature_ids`, `Catalog::area_statistics`,
  `Catalog::compliance_percent`
- `render_lsp_feature_catalog_module`, `render_lsp_fallback_module`
- `render_dap_feature_catalog_module`, `render_dap_fallback_module`
- `DEFAULT_DAP_FEATURES`
