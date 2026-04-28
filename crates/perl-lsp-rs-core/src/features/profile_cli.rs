#![warn(missing_docs)]
//! CLI profile-token parsing for Perl LSP feature governance.
//!
//! This microcrate owns one responsibility: parse user-facing profile tokens
//! and expose diagnostics helpers for invalid values.

use std::fmt;

pub use crate::features::policy::FeatureProfile;
use crate::features::profile::parse_profile_token;

/// Parse a `--feature-profile` argument into a runtime policy.
///
/// Returns a structured error when the raw token is unknown. The error includes
/// the current supported token set for CLI/user-facing diagnostics.
pub fn parse_feature_profile_arg(
    raw_profile: &str,
) -> Result<FeatureProfile, UnsupportedFeatureProfileError> {
    match parse_profile_token(raw_profile) {
        Some(kind) => Ok(FeatureProfile::from_kind(kind)),
        None => Err(UnsupportedFeatureProfileError { raw_profile: raw_profile.to_string() }),
    }
}

/// Parse a profile argument and fall back to `FeatureProfile::current()`.
pub fn parse_feature_profile_arg_or_current(raw_profile: &str) -> FeatureProfile {
    parse_feature_profile_arg(raw_profile).unwrap_or_else(|_| FeatureProfile::current())
}

/// Canonical profile label for logs and diagnostics.
pub const fn feature_profile_label(profile: FeatureProfile) -> &'static str {
    profile.as_str()
}

/// Supported CLI tokens accepted by the profile parser.
pub const fn feature_profile_supported_tokens() -> &'static [&'static str] {
    FeatureProfile::supported_cli_profiles()
}

/// Structured parse error for invalid profile tokens.
#[derive(Debug)]
pub struct UnsupportedFeatureProfileError {
    /// Raw token that could not be resolved.
    pub raw_profile: String,
}

impl UnsupportedFeatureProfileError {
    /// Human-friendly error message with support list.
    pub fn message(&self) -> String {
        let supported = feature_profile_supported_tokens().join(", ");
        format!("Invalid feature profile: {}. Supported: {}", self.raw_profile, supported)
    }
}

impl fmt::Display for UnsupportedFeatureProfileError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.message())
    }
}

impl std::error::Error for UnsupportedFeatureProfileError {}

#[cfg(test)]
mod tests {
    use super::{
        FeatureProfile, UnsupportedFeatureProfileError, feature_profile_label,
        feature_profile_supported_tokens, parse_feature_profile_arg,
        parse_feature_profile_arg_or_current,
    };
    use perl_tdd_support::{must, must_err};

    #[test]
    fn parse_feature_profile_accepts_known_aliases() {
        let profile = must(parse_feature_profile_arg("ga_lock"));
        assert_eq!(profile.as_str(), "ga-lock");

        let profile = must(parse_feature_profile_arg("Prod"));
        assert_eq!(profile.as_str(), "production");

        let profile = must(parse_feature_profile_arg("  ALL  "));
        assert_eq!(profile.as_str(), "all");
    }

    #[test]
    fn parse_unknown_profile_falls_back_to_current() {
        let profile = parse_feature_profile_arg_or_current("unknown-profile");
        assert_eq!(profile, FeatureProfile::current());
    }

    #[test]
    fn supported_tokens_contain_expected() {
        let supported = feature_profile_supported_tokens();
        assert!(supported.contains(&"auto"));
        assert!(supported.contains(&"ga"));
        assert!(supported.contains(&"prod"));
        assert!(supported.contains(&"all"));
    }

    // ── Error diagnostics ───────────────────────────────────────────

    #[test]
    fn parse_feature_profile_arg_returns_error_for_unknown() {
        let err = must_err(parse_feature_profile_arg("bogus"));
        assert!(err.raw_profile == "bogus", "error should capture the raw profile token");
    }

    #[test]
    fn unsupported_error_message_contains_raw_token() {
        let err = UnsupportedFeatureProfileError { raw_profile: "xyzzy".to_string() };
        let msg = err.message();
        assert!(msg.contains("xyzzy"), "error message should contain the raw token");
    }

    #[test]
    fn unsupported_error_message_lists_supported_tokens() {
        let err = UnsupportedFeatureProfileError { raw_profile: "bad".to_string() };
        let msg = err.message();
        assert!(msg.contains("auto"), "error message should list 'auto'");
        assert!(msg.contains("prod"), "error message should list 'prod'");
        assert!(msg.contains("all"), "error message should list 'all'");
    }

    #[test]
    fn unsupported_error_display_matches_message() {
        let err = UnsupportedFeatureProfileError { raw_profile: "nope".to_string() };
        let display = format!("{err}");
        assert_eq!(display, err.message());
    }

    #[test]
    fn unsupported_error_is_std_error() {
        let err: Box<dyn std::error::Error> =
            Box::new(UnsupportedFeatureProfileError { raw_profile: "test".to_string() });
        let msg = format!("{err}");
        assert!(msg.contains("test"));
    }

    // ── feature_profile_label ───────────────────────────────────────

    #[test]
    fn feature_profile_label_returns_canonical_name() {
        assert_eq!(feature_profile_label(FeatureProfile::GaLock), "ga-lock");
        assert_eq!(feature_profile_label(FeatureProfile::Production), "production");
        assert_eq!(feature_profile_label(FeatureProfile::All), "all");
    }

    // ── parse_feature_profile_arg_or_current with valid input ───────

    #[test]
    fn parse_feature_profile_arg_or_current_accepts_valid() {
        let profile = parse_feature_profile_arg_or_current("all");
        assert_eq!(profile, FeatureProfile::All);
    }

    #[test]
    fn parse_feature_profile_arg_or_current_with_whitespace() {
        let profile = parse_feature_profile_arg_or_current("  ga  ");
        assert_eq!(profile, FeatureProfile::GaLock);
    }
}
