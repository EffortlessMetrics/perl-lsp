//! Kwalitee evaluation profiles.
//!
//! A profile selects *which* indicators are mandatory and how strictly missing
//! evidence is treated. The three profiles form a widening ladder:
//!
//! - [`KwaliteeProfile::Pr`] — fast, no release artifacts required. Release-area
//!   indicators are reported [`NotApplicable`](crate::IndicatorStatus::NotApplicable).
//! - [`KwaliteeProfile::Release`] — strict, requires a `--dist` directory. Every
//!   PR indicator plus the release-archive contract must pass.
//! - [`KwaliteeProfile::Nightly`] — the same mandatory floor as `Pr` plus a set
//!   of broad, receipt-heavy **advisory** indicators (formatter corpus
//!   idempotence, native-critic false positives, perltidy/perlcritic
//!   external-only gaps) that only run under this profile.

use std::fmt;
use std::str::FromStr;

use serde::{Deserialize, Serialize};

/// Which Kwalitee profile is being evaluated.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum KwaliteeProfile {
    /// Fast per-PR profile. Release-artifact indicators are not applicable.
    Pr,
    /// Strict release profile. Requires a populated `dist` directory.
    Release,
    /// Broad nightly profile. Same mandatory floor as `Pr`, plus nightly-only
    /// receipt-heavy advisory indicators.
    Nightly,
}

impl KwaliteeProfile {
    /// Lowercase wire/display name (`"pr"`, `"release"`, `"nightly"`).
    pub fn as_str(self) -> &'static str {
        match self {
            KwaliteeProfile::Pr => "pr",
            KwaliteeProfile::Release => "release",
            KwaliteeProfile::Nightly => "nightly",
        }
    }

    /// Whether release-archive contract indicators are in scope for this profile.
    ///
    /// Only [`KwaliteeProfile::Release`] treats release-archive indicators as
    /// mandatory; the other profiles report them as not-applicable.
    pub fn requires_release_artifacts(self) -> bool {
        matches!(self, KwaliteeProfile::Release)
    }
}

impl fmt::Display for KwaliteeProfile {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl FromStr for KwaliteeProfile {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.trim().to_ascii_lowercase().as_str() {
            "pr" => Ok(KwaliteeProfile::Pr),
            "release" => Ok(KwaliteeProfile::Release),
            "nightly" => Ok(KwaliteeProfile::Nightly),
            other => {
                Err(format!("unknown Kwalitee profile `{other}` (expected pr|release|nightly)"))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roundtrips_through_str() {
        for p in [KwaliteeProfile::Pr, KwaliteeProfile::Release, KwaliteeProfile::Nightly] {
            assert_eq!(KwaliteeProfile::from_str(p.as_str()), Ok(p));
        }
    }

    #[test]
    fn parse_is_case_insensitive_and_trims() {
        assert_eq!(KwaliteeProfile::from_str("  Release "), Ok(KwaliteeProfile::Release));
    }

    #[test]
    fn unknown_profile_is_an_error() {
        assert!(KwaliteeProfile::from_str("prod").is_err());
    }

    #[test]
    fn only_release_requires_artifacts() {
        assert!(KwaliteeProfile::Release.requires_release_artifacts());
        assert!(!KwaliteeProfile::Pr.requires_release_artifacts());
        assert!(!KwaliteeProfile::Nightly.requires_release_artifacts());
    }

    #[test]
    fn serializes_snake_case() {
        let j = serde_json::to_string(&KwaliteeProfile::Nightly).expect("serialize");
        assert_eq!(j, "\"nightly\"");
    }
}
