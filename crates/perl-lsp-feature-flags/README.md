# perl-lsp-feature-flags

Microcrate for LSP feature flag modeling used by protocol capability construction,
runtime profile selection, and BDD/interoperability reporting.

## Purpose

This crate owns the canonical shape and behavior of capability flags:

- `BuildFlags` — runtime/build-time capability toggles used by the protocol layer.
- `AdvertisedFeatures` — client-advertising projection of runtime capability flags.
- `BuildFlags::to_advertised_features` — projection from build flags to advertised features.
- `BuildFlags::to_feature_ids` — feature-id projection used by BDD tooling.
- `BuildFlags::{production,ga_lock,all}` — canonical feature profiles.

## API

All API is intentionally small and stable:

- [BuildFlags struct](https://docs.rs/perl-lsp-feature-flags/latest/perl_lsp_feature_flags/struct.BuildFlags.html)
- [AdvertisedFeatures struct](https://docs.rs/perl-lsp-feature-flags/latest/perl_lsp_feature_flags/struct.AdvertisedFeatures.html)
- `to_advertised_features` and `to_feature_ids`
- `production`, `ga_lock`, and `all` profile constructors

## Interoperability

- `perl-lsp-protocol` re-exports `BuildFlags` and `AdvertisedFeatures`, so existing
  import paths continue to work while still using this microcrate as the source of truth.
- `perl-lsp-feature-policy` uses this crate to translate profile decisions into
  concrete protocol capability flags.

