# perl-lsp-feature-profile

Microcrate for canonical LSP feature profile identifiers and parsing behavior.

## Purpose

- Centralize accepted profile tokens and aliases.
- Provide stable labels used across CLI, runtime defaults, and reporting.
- Keep profile name and alias policy independent of policy decision logic.

## API

- `FeatureProfileKind` — canonical runtime profile domain.
- `FeatureProfileKind::from_str_name` — parse aliases (`ga`, `ga-lock`, `ga_lock`, `prod`, `production`, `all`, `auto`).
- `FeatureProfileKind::from_ga_lock_enabled` — map compile-time GA-lock feature into profile.
- `FeatureProfileKind::current` — default profile from enabled Cargo features.
- `FeatureProfileKind::all` — all supported canonical profiles.
- `FeatureProfileKind::as_str` — canonical label (`ga-lock`, `production`, `all`).
- `supported_cli_profiles` — tokens accepted by CLI/config validation.

