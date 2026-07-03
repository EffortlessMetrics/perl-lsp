//! The Kwalitee receipt: the durable, serializable evaluation result.
//!
//! A [`KwaliteeReceipt`] is the product artifact — a versioned JSON document
//! (`kind = "perl_kwalitee"`, `schema_version = 1`) plus a Markdown rendering
//! for humans. It carries the profile, the derived score/verdict, roll-up
//! counts, and the full indicator table.

use serde::{Deserialize, Serialize};

use crate::indicator::{IndicatorStatus, KwaliteeIndicator};
use crate::profile::KwaliteeProfile;

/// Receipt `kind` discriminator.
pub const RECEIPT_KIND: &str = "perl_kwalitee";
/// Current receipt schema version.
pub const SCHEMA_VERSION: u32 = 1;

/// Overall verdict for an evaluation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum KwaliteeVerdict {
    /// All applicable mandatory indicators pass; no soft concerns.
    Pass,
    /// No mandatory failure, but a soft concern remains (warnings / unverified).
    Warn,
    /// At least one mandatory indicator failed (or was unverified under strict).
    Fail,
}

impl KwaliteeVerdict {
    /// Lowercase wire/display name.
    pub fn as_str(self) -> &'static str {
        match self {
            KwaliteeVerdict::Pass => "pass",
            KwaliteeVerdict::Warn => "warn",
            KwaliteeVerdict::Fail => "fail",
        }
    }

    /// Uppercase label for report headers.
    pub fn label(self) -> &'static str {
        match self {
            KwaliteeVerdict::Pass => "PASS",
            KwaliteeVerdict::Warn => "WARN",
            KwaliteeVerdict::Fail => "FAIL",
        }
    }

    /// Whether this verdict should be treated as a gate failure (used by
    /// `check` under a given strictness — a `Fail` always blocks).
    pub fn is_failure(self) -> bool {
        matches!(self, KwaliteeVerdict::Fail)
    }
}

/// A complete Kwalitee evaluation result.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct KwaliteeReceipt {
    /// Always [`RECEIPT_KIND`].
    pub kind: String,
    /// Always [`SCHEMA_VERSION`].
    pub schema_version: u32,
    /// Caller-supplied timestamp (RFC 3339). Empty when not provided.
    pub generated_at: String,
    /// Caller-supplied git commit the evaluation reflects. Empty when unknown.
    pub commit: String,
    /// The profile that was evaluated.
    pub profile: KwaliteeProfile,
    /// Numeric score, 0..=100.
    pub score: u8,
    /// Overall verdict.
    pub verdict: KwaliteeVerdict,
    /// Whether every mandatory indicator passed.
    pub mandatory_passed: bool,
    /// Count of mandatory indicators that failed.
    pub mandatory_failed_count: usize,
    /// Count of mandatory indicators that are unverified. Distinct from
    /// `mandatory_failed_count`: under `--strict` these drive the `fail`
    /// verdict, but they are not counted as failures otherwise.
    pub mandatory_unverified_count: usize,
    /// Count of indicators in `warn`.
    pub warning_count: usize,
    /// Count of indicators in `unverified`.
    pub unverified_count: usize,
    /// The full indicator table, in catalog order.
    pub indicators: Vec<KwaliteeIndicator>,
}

impl KwaliteeReceipt {
    /// Serialize to pretty JSON.
    pub fn to_json_pretty(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string_pretty(self)
    }

    /// Indicators that are mandatory and applicable, in catalog order.
    pub fn mandatory_indicators(&self) -> impl Iterator<Item = &KwaliteeIndicator> {
        self.indicators.iter().filter(|i| i.mandatory && i.status.is_applicable())
    }

    /// Render a human-readable Markdown report.
    pub fn to_markdown(&self) -> String {
        let mut out = String::new();
        out.push_str("# Perl Kwalitee Report\n\n");
        out.push_str(&format!("Verdict: {}\n\n", self.verdict.label()));
        out.push_str(&format!("Score: {}/100\n\n", self.score));
        out.push_str(&format!("Profile: {}\n\n", self.profile));
        if !self.commit.is_empty() {
            out.push_str(&format!("Commit: {}\n\n", self.commit));
        }
        if !self.generated_at.is_empty() {
            out.push_str(&format!("Generated: {}\n\n", self.generated_at));
        }

        let mandatory: Vec<&KwaliteeIndicator> =
            self.indicators.iter().filter(|i| i.mandatory).collect();
        let advisory: Vec<&KwaliteeIndicator> =
            self.indicators.iter().filter(|i| !i.mandatory).collect();

        render_table(&mut out, "Mandatory indicators", &mandatory);
        if !advisory.is_empty() {
            out.push('\n');
            render_table(&mut out, "Advisory indicators", &advisory);
        }

        out
    }
}

/// Render a Markdown table for a set of indicators under a heading.
fn render_table(out: &mut String, heading: &str, indicators: &[&KwaliteeIndicator]) {
    out.push_str(&format!("## {heading}\n\n"));
    if indicators.is_empty() {
        out.push_str("_none_\n");
        return;
    }
    out.push_str("| Indicator | Status | Evidence |\n");
    out.push_str("|---|---|---|\n");
    for ind in indicators {
        let evidence = if ind.evidence.is_empty() {
            String::from("—")
        } else {
            ind.evidence
                .iter()
                .map(|e| format!("`{}`", sanitize_cell(&e.value)))
                .collect::<Vec<_>>()
                .join("<br>")
        };
        out.push_str(&format!("| {} | {} | {} |\n", ind.id, status_cell(ind.status), evidence));
    }
    // Remediation notes for anything not passing.
    let failing: Vec<&&KwaliteeIndicator> = indicators
        .iter()
        .filter(|i| {
            matches!(
                i.status,
                IndicatorStatus::Fail | IndicatorStatus::Warn | IndicatorStatus::Unverified
            )
        })
        .collect();
    if !failing.is_empty() {
        out.push_str("\n### Remediation\n\n");
        for ind in failing {
            if let Some(r) = &ind.remediation {
                out.push_str(&format!("- **{}** ({}): {}\n", ind.id, ind.status.as_str(), r));
            }
        }
    }
}

fn status_cell(status: IndicatorStatus) -> &'static str {
    status.as_str()
}

/// Sanitize a free-form evidence value for a Markdown table cell wrapped in a
/// backtick code span. Evidence values can carry arbitrary error text
/// (`Display` of a gate error), so collapse newlines to spaces, neutralize
/// backticks (which would break the code span), and escape the cell separator.
fn sanitize_cell(value: &str) -> String {
    value.replace(['\r', '\n'], " ").replace('`', "'").replace('|', "\\|")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::indicator::EvidenceRef;

    fn sample_receipt() -> KwaliteeReceipt {
        KwaliteeReceipt {
            kind: RECEIPT_KIND.to_string(),
            schema_version: SCHEMA_VERSION,
            generated_at: "2026-07-03T00:00:00Z".to_string(),
            commit: "abcdef1".to_string(),
            profile: KwaliteeProfile::Release,
            score: 100,
            verdict: KwaliteeVerdict::Pass,
            mandatory_passed: true,
            mandatory_failed_count: 0,
            mandatory_unverified_count: 0,
            warning_count: 0,
            unverified_count: 0,
            indicators: vec![
                KwaliteeIndicator {
                    id: "release.no_external_tooling".to_string(),
                    area: "release".to_string(),
                    title: "no external tooling".to_string(),
                    mandatory: true,
                    status: IndicatorStatus::Pass,
                    score_weight: 8,
                    evidence: vec![EvidenceRef::command(
                        "cargo xtask release artifact-check --dist dist",
                    )],
                    remediation: None,
                },
                KwaliteeIndicator {
                    id: "critic.run_critic_registry_parity".to_string(),
                    area: "critic".to_string(),
                    title: "runCritic parity".to_string(),
                    mandatory: false,
                    status: IndicatorStatus::Unverified,
                    score_weight: 7,
                    evidence: vec![],
                    remediation: Some("Land #3303.".to_string()),
                },
            ],
        }
    }

    #[test]
    fn json_roundtrips() {
        let r = sample_receipt();
        let json = r.to_json_pretty().expect("serialize");
        let back: KwaliteeReceipt = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(r, back);
    }

    #[test]
    fn json_has_stable_envelope() {
        let r = sample_receipt();
        let v: serde_json::Value = serde_json::to_value(&r).expect("value");
        assert_eq!(v["kind"], "perl_kwalitee");
        assert_eq!(v["schema_version"], 1);
        assert_eq!(v["verdict"], "pass");
        assert_eq!(v["profile"], "release");
    }

    #[test]
    fn markdown_cell_sanitizes_newlines_and_backticks() {
        let mut r = sample_receipt();
        r.indicators[0].evidence =
            vec![EvidenceRef::new("note", "failed:\n  detail with `backtick` and | pipe")];
        let md = r.to_markdown();
        // No raw newline may leak inside the table row for that evidence.
        assert!(!md.contains("failed:\n"));
        assert!(md.contains("failed:   detail with 'backtick' and \\| pipe"));
    }

    #[test]
    fn markdown_contains_heading_and_rows() {
        let md = sample_receipt().to_markdown();
        assert!(md.contains("# Perl Kwalitee Report"));
        assert!(md.contains("Verdict: PASS"));
        assert!(md.contains("## Mandatory indicators"));
        assert!(md.contains("release.no_external_tooling"));
        assert!(md.contains("## Advisory indicators"));
        // Remediation for the unverified advisory row is surfaced.
        assert!(md.contains("Land #3303."));
    }
}
