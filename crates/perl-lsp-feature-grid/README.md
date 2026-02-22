# perl-lsp-feature-grid

Shared BDD grid and feature-profile reporting API for Perl LSP.

This microcrate is intentionally narrow:

- re-export canonical feature contracts and BDD rows
- provide profile-aware feature-grid serialization
- keep JSON reporting shape stable for tooling and CI

## API highlights

- `to_json()` returns the historical catalog JSON payload used by CLI and snapshots.
- `to_json_for_profile(profile)` returns the same payload, plus profile-specific advertised
  feature selection and compliance calculation.
- `to_json_for_profiles(profiles)` serializes the same payload for an explicit profile set.
- `to_json_for_all_profiles()` emits all canonical profiles at once for matrix-style
  interoperability checks.
- `compliance_percent_for_profile(profile)` exposes profile-specific compliance math as a
  first-class API.
- `FeatureProfile` and catalog compatibility exports are re-exported for interoperability.
- `FEATURE_GRID_COLUMNS` exposes the canonical BDD header order.
