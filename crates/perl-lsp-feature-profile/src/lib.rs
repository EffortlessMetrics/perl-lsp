#![warn(missing_docs)]
//! Canonical feature profile definitions used by runtime profile selection, CLI parsing,
//! and downstream reporting tools.
//!
//! This crate intentionally keeps profile naming and canonicalization rules in one
//! place so all LSP components (and external tooling) can share the same behavior
//! without duplicating alias logic.

pub use perl_lsp_feature_contracts::{
    FeatureProfileKind, FeatureProfileSpec, feature_profile_specs,
};

/// Return every supported CLI token for profile parsing/validation.
pub const fn supported_cli_profiles() -> &'static [&'static str] {
    FeatureProfileKind::supported_cli_profiles()
}

/// Parse a raw profile token into canonical form.
pub fn from_str_name(s: &str) -> Option<FeatureProfileKind> {
    FeatureProfileKind::from_str_name(s)
}

/// Parse and normalize user input with trimming and legacy delimiter compatibility.
pub fn parse_profile_token(raw_profile: &str) -> Option<FeatureProfileKind> {
    let normalized = raw_profile.trim().to_ascii_lowercase().replace('_', "-");

    from_str_name(&normalized)
}

#[cfg(test)]
mod tests {
    use super::FeatureProfileKind;

    #[test]
    fn supported_profiles_contains_expected_values() {
        let supported = super::supported_cli_profiles();
        assert!(supported.contains(&"auto"));
        assert!(supported.contains(&"ga"));
        assert!(supported.contains(&"ga_lock"));
        assert!(supported.contains(&"ga-lock"));
        assert!(supported.contains(&"prod"));
        assert!(supported.contains(&"production"));
        assert!(supported.contains(&"all"));
    }

    #[test]
    fn canonical_names_are_stable() {
        assert_eq!(FeatureProfileKind::GaLock.as_str(), "ga-lock");
        assert_eq!(FeatureProfileKind::Production.as_str(), "production");
        assert_eq!(FeatureProfileKind::All.as_str(), "all");
    }

    #[test]
    fn aliases_resolve_to_known_profiles() {
        assert_eq!(
            FeatureProfileKind::from_str_name("auto"),
            Some(FeatureProfileKind::current())
        );
        assert_eq!(
            FeatureProfileKind::from_str_name("ga-lock"),
            Some(FeatureProfileKind::GaLock)
        );
        assert_eq!(
            FeatureProfileKind::from_str_name("ga"),
            Some(FeatureProfileKind::GaLock)
        );
        assert_eq!(
            FeatureProfileKind::from_str_name("ga_lock"),
            Some(FeatureProfileKind::GaLock)
        );
        assert_eq!(
            FeatureProfileKind::from_str_name("prod"),
            Some(FeatureProfileKind::Production)
        );
        assert_eq!(
            FeatureProfileKind::from_str_name("all"),
            Some(FeatureProfileKind::All)
        );
        assert_eq!(FeatureProfileKind::from_str_name("unknown"), None);
    }

    #[test]
    fn normalized_profiles_keep_legacy_underscores() {
        assert_eq!(
            super::parse_profile_token("ga_lock"),
            Some(FeatureProfileKind::GaLock)
        );
    }

    // ── parse_profile_token normalization ────────────────────────────

    #[test]
    fn parse_profile_token_trims_whitespace() {
        assert_eq!(
            super::parse_profile_token("  all  "),
            Some(FeatureProfileKind::All)
        );
        assert_eq!(
            super::parse_profile_token("\tprod\n"),
            Some(FeatureProfileKind::Production)
        );
    }

    #[test]
    fn parse_profile_token_lowercases() {
        assert_eq!(
            super::parse_profile_token("ALL"),
            Some(FeatureProfileKind::All)
        );
        assert_eq!(
            super::parse_profile_token("Prod"),
            Some(FeatureProfileKind::Production)
        );
        assert_eq!(
            super::parse_profile_token("GA-LOCK"),
            Some(FeatureProfileKind::GaLock)
        );
    }

    #[test]
    fn parse_profile_token_normalizes_underscore_to_hyphen() {
        assert_eq!(
            super::parse_profile_token("GA_LOCK"),
            Some(FeatureProfileKind::GaLock)
        );
    }

    #[test]
    fn parse_profile_token_rejects_empty() {
        assert!(super::parse_profile_token("").is_none());
    }

    #[test]
    fn parse_profile_token_rejects_unknown() {
        assert!(super::parse_profile_token("debug").is_none());
        assert!(super::parse_profile_token("minimal").is_none());
    }

    #[test]
    fn parse_profile_token_resolves_auto() {
        assert_eq!(
            super::parse_profile_token("auto"),
            Some(FeatureProfileKind::current())
        );
        assert_eq!(
            super::parse_profile_token("AUTO"),
            Some(FeatureProfileKind::current())
        );
    }

    // ── from_str_name delegates ─────────────────────────────────────

    #[test]
    fn from_str_name_delegates_to_profile_kind() {
        assert_eq!(
            super::from_str_name("all"),
            FeatureProfileKind::from_str_name("all")
        );
        assert_eq!(super::from_str_name("bogus"), None);
    }
}
