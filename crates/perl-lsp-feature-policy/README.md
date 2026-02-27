# perl-lsp-feature-policy

Microcrate for LSP runtime feature-profile policy.

## Purpose

This crate maps high-level feature profiles (GA lock, production, all) to:

- `BuildFlags` values consumed by `perl-lsp-protocol` (re-exported from `perl-lsp-feature-flags`)
- Canonical feature IDs from the catalog-backed feature contract source

## API

- `FeatureProfile` — profile enum used for capability selection
- `FeatureProfile::current` — profile selected from compiled-in crate features (`lsp-ga-lock` support)
- `FeatureProfile::build_flags` — base `BuildFlags` for a profile
- `FeatureProfile::runtime_flags` — runtime flags with tool availability toggles
- `FeatureProfile::advertised_features` — direct conversion to advertised feature bundle
- `FeatureProfile::as_str` — canonical profile label for diagnostics and logs
- `FeatureProfile::supported_cli_profiles` — canonical CLI tokens accepted by profile parsing
- `flags_for_profile` — compatibility API for base flags
- `flags_for_runtime` — compatibility API for runtime flags
- `feature_ids_from_flags` — map flags back to LSP feature IDs
- `catalog_advertised_feature_ids` — intersection of policy IDs with catalog advertisements

Parsing and aliasing behavior is centralized in `perl-lsp-feature-profile` and
re-exported through this crate via the `FeatureProfile` API.
