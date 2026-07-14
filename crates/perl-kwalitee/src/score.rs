//! Scoring and verdict derivation from a set of evaluated indicators.
//!
//! The numeric score is deliberately secondary to the mandatory-indicator
//! table; the important product signal is *which mandatory indicators failed*,
//! not the exact number. The scoring rules (per the design):
//!
//! - a mandatory `Fail` ⇒ verdict `Fail`, score capped at 89;
//! - a mandatory `Unverified` ⇒ `Fail` under `--strict`, otherwise `Warn`;
//! - warnings only ⇒ verdict `Warn`, score banded to 90..=99;
//! - all applicable indicators pass ⇒ score 100.
//!
//! [`NotApplicable`](crate::IndicatorStatus::NotApplicable) indicators are
//! excluded from scoring entirely.

use crate::indicator::{IndicatorStatus, KwaliteeIndicator};
use crate::receipt::KwaliteeVerdict;

/// Aggregate scoring outcome derived from a set of indicators.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Scored {
    pub score: u8,
    pub verdict: KwaliteeVerdict,
    pub mandatory_passed: bool,
    pub mandatory_failed_count: usize,
    pub mandatory_unverified_count: usize,
    pub warning_count: usize,
    pub unverified_count: usize,
}

/// Fraction of an indicator's weight earned by its status.
fn weight_factor(status: IndicatorStatus) -> f64 {
    match status {
        IndicatorStatus::Pass => 1.0,
        IndicatorStatus::Warn => 0.5,
        // Fail / Unverified earn nothing.
        IndicatorStatus::Fail | IndicatorStatus::Unverified => 0.0,
        // NotApplicable is excluded from the denominator before this is called.
        IndicatorStatus::NotApplicable => 0.0,
    }
}

/// Compute the score and verdict for a set of indicators under a strictness.
pub(crate) fn score(indicators: &[KwaliteeIndicator], strict: bool) -> Scored {
    let applicable: Vec<&KwaliteeIndicator> =
        indicators.iter().filter(|i| i.status.is_applicable()).collect();

    let total_weight: f64 = applicable.iter().map(|i| f64::from(i.score_weight)).sum();
    let earned_weight: f64 =
        applicable.iter().map(|i| f64::from(i.score_weight) * weight_factor(i.status)).sum();

    let raw = if total_weight <= 0.0 {
        100u8
    } else {
        // Round to nearest, clamp into u8.
        ((earned_weight / total_weight) * 100.0).round().clamp(0.0, 100.0) as u8
    };

    let mandatory_failed_count =
        applicable.iter().filter(|i| i.mandatory && i.status == IndicatorStatus::Fail).count();
    let mandatory_unverified_count = applicable
        .iter()
        .filter(|i| i.mandatory && i.status == IndicatorStatus::Unverified)
        .count();
    let warning_count = applicable.iter().filter(|i| i.status == IndicatorStatus::Warn).count();
    let unverified_count =
        applicable.iter().filter(|i| i.status == IndicatorStatus::Unverified).count();
    let nonmandatory_fail_count =
        applicable.iter().filter(|i| !i.mandatory && i.status == IndicatorStatus::Fail).count();

    // A hard fail: any mandatory Fail, or a mandatory Unverified under --strict.
    let hard_fail = mandatory_failed_count > 0 || (strict && mandatory_unverified_count > 0);

    // A warn state: no hard fail, but some soft concern remains — an explicit
    // Warn, a mandatory Unverified tolerated under non-strict, or any
    // non-mandatory Fail/Unverified.
    let soft_concern = warning_count > 0
        || (!strict && mandatory_unverified_count > 0)
        || nonmandatory_fail_count > 0
        || (unverified_count > 0 && !hard_fail);

    let (verdict, score) = if hard_fail {
        (KwaliteeVerdict::Fail, raw.min(89))
    } else if soft_concern {
        (KwaliteeVerdict::Warn, raw.clamp(90, 99))
    } else {
        (KwaliteeVerdict::Pass, 100)
    };

    // mandatory_passed: every applicable mandatory indicator is *Pass*. A
    // mandatory Warn (e.g. a stale-but-passing receipt downgraded to Warn) is a
    // non-pass and must make this false, matching the documented contract
    // ("every mandatory indicator passed").
    let mandatory_passed =
        applicable.iter().filter(|i| i.mandatory).all(|i| i.status == IndicatorStatus::Pass);

    Scored {
        score,
        verdict,
        mandatory_passed,
        mandatory_failed_count,
        mandatory_unverified_count,
        warning_count,
        unverified_count,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::indicator::IndicatorStatus;

    fn ind(id: &str, mandatory: bool, weight: u8, status: IndicatorStatus) -> KwaliteeIndicator {
        KwaliteeIndicator {
            id: id.to_string(),
            area: "test".to_string(),
            title: id.to_string(),
            mandatory,
            status,
            score_weight: weight,
            evidence: vec![],
            remediation: None,
        }
    }

    #[test]
    fn all_pass_is_100_and_pass() {
        let inds = vec![
            ind("a", true, 10, IndicatorStatus::Pass),
            ind("b", true, 5, IndicatorStatus::Pass),
        ];
        let s = score(&inds, false);
        assert_eq!(s.score, 100);
        assert_eq!(s.verdict, KwaliteeVerdict::Pass);
        assert!(s.mandatory_passed);
    }

    #[test]
    fn mandatory_fail_caps_at_89_and_fails() {
        let inds = vec![
            ind("a", true, 10, IndicatorStatus::Pass),
            ind("b", true, 1, IndicatorStatus::Fail),
        ];
        let s = score(&inds, false);
        assert!(s.score <= 89, "score was {}", s.score);
        assert_eq!(s.verdict, KwaliteeVerdict::Fail);
        assert!(!s.mandatory_passed);
        assert_eq!(s.mandatory_failed_count, 1);
    }

    #[test]
    fn mandatory_unverified_fails_under_strict_warns_otherwise() {
        let inds = vec![
            ind("a", true, 10, IndicatorStatus::Pass),
            ind("b", true, 5, IndicatorStatus::Unverified),
        ];
        let strict = score(&inds, true);
        assert_eq!(strict.verdict, KwaliteeVerdict::Fail);
        assert!(strict.score <= 89);

        let lax = score(&inds, false);
        assert_eq!(lax.verdict, KwaliteeVerdict::Warn);
        assert!((90..=99).contains(&lax.score), "score was {}", lax.score);
    }

    #[test]
    fn warnings_only_band_90_99() {
        let inds = vec![
            ind("a", true, 10, IndicatorStatus::Pass),
            ind("b", false, 5, IndicatorStatus::Warn),
        ];
        let s = score(&inds, true);
        assert_eq!(s.verdict, KwaliteeVerdict::Warn);
        assert!((90..=99).contains(&s.score), "score was {}", s.score);
        // mandatory all passed even though a non-mandatory warned.
        assert!(s.mandatory_passed);
    }

    #[test]
    fn mandatory_warn_makes_mandatory_passed_false() {
        // A mandatory indicator in Warn is a non-pass: mandatory_passed must be
        // false even though the verdict is only Warn (not Fail).
        let inds = vec![
            ind("a", true, 10, IndicatorStatus::Pass),
            ind("b", true, 5, IndicatorStatus::Warn),
        ];
        let s = score(&inds, false);
        assert_eq!(s.verdict, KwaliteeVerdict::Warn);
        assert!(!s.mandatory_passed, "mandatory Warn must not report mandatory_passed");
    }

    #[test]
    fn not_applicable_excluded_from_scoring() {
        let inds = vec![
            ind("a", true, 10, IndicatorStatus::Pass),
            ind("b", true, 100, IndicatorStatus::NotApplicable),
        ];
        let s = score(&inds, true);
        assert_eq!(s.score, 100);
        assert_eq!(s.verdict, KwaliteeVerdict::Pass);
    }

    #[test]
    fn empty_or_all_na_scores_100() {
        assert_eq!(score(&[], true).score, 100);
        let inds = vec![ind("a", true, 10, IndicatorStatus::NotApplicable)];
        assert_eq!(score(&inds, true).score, 100);
    }
}
