//! CI failure classifier — `cargo xtask failure-classifier`
//!
//! Classifies failed CI runs before routing labels/actions are applied.

use std::fs;
use std::path::{Path, PathBuf};

use color_eyre::eyre::{Context, Result};
use serde::{Deserialize, Serialize};

const CHECK_NAME: &str = "failure-classifier";
const SCHEMA_VERSION: &str = "1";

#[derive(Debug, Clone)]
pub struct FailureClassifierConfig {
    pub snapshot: Option<PathBuf>,
    pub receipt: Option<PathBuf>,
    pub fixture: Option<PathBuf>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FailureReceipt {
    /// Matches the schema `check` constant `"failure-classifier"`.
    pub check: String,
    /// Schema version for forward compatibility.
    pub schema_version: String,
    /// Triggering event context.
    pub event: String,
    /// Overall gate verdict: `"pass"` when no PR-owned failure detected.
    pub verdict: String,
    pub signature: String,
    pub affected_prs: Vec<u64>,
    pub master_sha: Option<String>,
    pub master_same_signature: bool,
    pub classification: FailureClassification,
    pub recommended_action: String,
    pub confidence: String,
    pub evidence: Vec<String>,
}

/// Classification values align with the `common-gate-receipt.schema.json` enum.
///
/// `CodeRegression` is the schema's name for what the classifier calls PR_OWNED
/// (a failure in code the PR changed). The schema uses `snake_case` values.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum FailureClassification {
    /// Failure in code changed by the PR (schema: `code_regression`).
    CodeRegression,
    StaleBase,
    MasterRed,
    InfraFailure,
    Flaky,
    Unknown,
}

#[derive(Debug, Clone, Deserialize, Default)]
struct SnapshotInput {
    #[serde(default)]
    pr: PullRequestInput,
    #[serde(default)]
    pr_checks: Vec<CheckInput>,
    #[serde(default)]
    master_checks: Vec<CheckInput>,
    #[serde(default)]
    merge_group_checks: Vec<CheckInput>,
    #[serde(default)]
    known_infra_signatures: Vec<String>,
    #[serde(default)]
    receipt_artifacts: Vec<String>,
    #[serde(default)]
    affected_prs: Vec<u64>,
}

#[derive(Debug, Clone, Deserialize, Default)]
struct PullRequestInput {
    number: Option<u64>,
    head_sha: Option<String>,
    master_sha: Option<String>,
    #[serde(default)]
    behind_master: bool,
    #[serde(default)]
    changed_files: Vec<String>,
}

#[derive(Debug, Clone, Deserialize, Default)]
struct CheckInput {
    name: Option<String>,
    signature: Option<String>,
    sha: Option<String>,
    conclusion: Option<String>,
    #[serde(default)]
    flaky: bool,
    #[serde(default)]
    files: Vec<String>,
    #[serde(default)]
    attempts: u32,
    #[serde(default)]
    recent_outcomes: Vec<String>,
}

pub fn run(config: FailureClassifierConfig) -> Result<()> {
    let input_path = config.fixture.or(config.snapshot).ok_or_else(|| {
        color_eyre::eyre::eyre!(
            "provide --snapshot <path> or --fixture <path> for failure classification"
        )
    })?;

    let input = load_input(&input_path)?;
    let receipt = classify(&input);
    let rendered = serde_json::to_string_pretty(&receipt)?;

    if let Some(receipt_path) = config.receipt {
        if let Some(parent) = receipt_path.parent() {
            fs::create_dir_all(parent).with_context(|| format!("creating {}", parent.display()))?;
        }
        fs::write(&receipt_path, &rendered)
            .with_context(|| format!("writing receipt {}", receipt_path.display()))?;
        println!("Wrote failure-classifier receipt: {}", receipt_path.display());
    } else {
        println!("{rendered}");
    }

    Ok(())
}

fn load_input(path: &Path) -> Result<SnapshotInput> {
    let raw = fs::read_to_string(path).with_context(|| format!("reading {}", path.display()))?;
    let input: SnapshotInput =
        serde_json::from_str(&raw).with_context(|| format!("parsing {}", path.display()))?;
    Ok(input)
}

fn classify(input: &SnapshotInput) -> FailureReceipt {
    let head_sha = input.pr.head_sha.as_deref();
    let failing_pr_checks = input
        .pr_checks
        .iter()
        .filter(|check| is_failure(check.conclusion.as_deref()))
        .filter(|check| match (head_sha, check.sha.as_deref()) {
            (Some(head), Some(check_sha)) => head == check_sha,
            (Some(_), None) => false,
            (None, _) => true,
        })
        .collect::<Vec<_>>();

    let signature = failing_pr_checks
        .first()
        .and_then(|check| check_signature(check))
        .unwrap_or_else(|| "unknown-signature".to_string());

    let master_same_signature = input
        .master_checks
        .iter()
        .filter(|check| is_failure(check.conclusion.as_deref()))
        .any(|master| check_signature(master).as_deref() == Some(signature.as_str()));

    let mut evidence = Vec::new();
    if let Some(pr_number) = input.pr.number {
        evidence.push(format!("PR #{pr_number} evaluated"));
    }
    if let Some(head) = &input.pr.head_sha {
        evidence.push(format!("PR head SHA: {head}"));
    }
    if let Some(master_sha) = &input.pr.master_sha {
        evidence.push(format!("Master SHA: {master_sha}"));
    }
    evidence.push(format!("Failing checks on PR head: {}", failing_pr_checks.len()));

    let has_head_evidence = !failing_pr_checks.is_empty();

    let classification = if !has_head_evidence {
        evidence.push("No failing check tied to current PR head SHA".to_string());
        FailureClassification::Unknown
    } else if is_infra_failure(input, &failing_pr_checks, &signature) {
        evidence.push("Matched known infra signature/pattern".to_string());
        FailureClassification::InfraFailure
    } else if master_same_signature {
        evidence.push("Master has matching failing signature".to_string());
        FailureClassification::MasterRed
    } else if input.pr.behind_master && master_green_for_signature(input, &signature) {
        evidence.push("PR head is behind master while master gate is green".to_string());
        FailureClassification::StaleBase
    } else if looks_flaky(input, &failing_pr_checks, &signature) {
        evidence.push("Observed flaky/retry pattern for failing signature".to_string());
        FailureClassification::Flaky
    } else if pr_owned_by_file_overlap(input, &failing_pr_checks) {
        evidence.push("Failed check references files changed by PR".to_string());
        FailureClassification::CodeRegression
    } else {
        evidence.push("No decisive routing signal found".to_string());
        FailureClassification::Unknown
    };

    let recommended_action = recommended_action(&classification).to_string();
    let confidence = confidence(&classification, has_head_evidence).to_string();

    let mut affected_prs =
        if input.affected_prs.is_empty() { Vec::new() } else { input.affected_prs.clone() };
    if let Some(number) = input.pr.number
        && !affected_prs.contains(&number)
    {
        affected_prs.push(number);
    }

    // verdict: "fail" when the failure is PR-owned (action required on PR);
    // "pass" when the failure is attributed to infra/master/stale-base/flaky.
    let verdict = match classification {
        FailureClassification::CodeRegression => "fail",
        _ => "pass",
    }
    .to_string();

    FailureReceipt {
        check: CHECK_NAME.to_string(),
        schema_version: SCHEMA_VERSION.to_string(),
        event: "pull_request".to_string(),
        verdict,
        signature,
        affected_prs,
        master_sha: input.pr.master_sha.clone(),
        master_same_signature,
        classification,
        recommended_action,
        confidence,
        evidence,
    }
}

fn check_signature(check: &CheckInput) -> Option<String> {
    check
        .signature
        .clone()
        .or_else(|| check.name.clone())
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
}

fn is_failure(conclusion: Option<&str>) -> bool {
    matches!(conclusion, Some("failure") | Some("timed_out") | Some("cancelled"))
}

fn is_success(conclusion: Option<&str>) -> bool {
    matches!(conclusion, Some("success") | Some("neutral") | Some("skipped"))
}

fn is_infra_failure(input: &SnapshotInput, failing: &[&CheckInput], signature: &str) -> bool {
    let known = input.known_infra_signatures.iter().any(|known| known == signature);
    if known {
        return true;
    }

    let mut all_artifacts = input.receipt_artifacts.clone();
    all_artifacts.extend(input.merge_group_checks.iter().filter_map(check_signature));

    let signatures = failing.iter().filter_map(|check| check_signature(check)).collect::<Vec<_>>();

    signatures.iter().any(|value| contains_infra_pattern(value))
        || all_artifacts.iter().any(|value| contains_infra_pattern(value))
}

fn contains_infra_pattern(value: &str) -> bool {
    let lowered = value.to_ascii_lowercase();
    lowered.contains("runner lost")
        || lowered.contains("service unavailable")
        || lowered.contains("network timeout")
        || lowered.contains("artifact upload")
        || lowered.contains("github outage")
}

fn master_green_for_signature(input: &SnapshotInput, signature: &str) -> bool {
    input
        .master_checks
        .iter()
        .filter(|check| check_signature(check).as_deref() == Some(signature))
        .any(|check| is_success(check.conclusion.as_deref()))
}

fn looks_flaky(input: &SnapshotInput, failing: &[&CheckInput], signature: &str) -> bool {
    let on_pr = failing.iter().any(|check| {
        check.flaky
            || check.attempts > 1
            || check.recent_outcomes.iter().any(|outcome| outcome == "success")
    });

    let on_master = input
        .master_checks
        .iter()
        .filter(|check| check_signature(check).as_deref() == Some(signature))
        .any(|check| check.flaky || check.attempts > 1);

    on_pr || on_master
}

fn pr_owned_by_file_overlap(input: &SnapshotInput, failing: &[&CheckInput]) -> bool {
    if input.pr.changed_files.is_empty() {
        return false;
    }

    failing.iter().any(|check| {
        check.files.iter().any(|file| input.pr.changed_files.iter().any(|changed| changed == file))
    })
}

fn recommended_action(classification: &FailureClassification) -> &'static str {
    match classification {
        // code_regression in schema maps to PR_OWNED semantics.
        FailureClassification::CodeRegression => "NEEDS_CI_FIX / builder",
        FailureClassification::StaleBase => "NEEDS_CASCADE_UPDATE",
        FailureClassification::MasterRed => "master incident / no PR-owned label",
        FailureClassification::InfraFailure => "infra/tooling route",
        FailureClassification::Flaky => "rerun/observe",
        FailureClassification::Unknown => "human classification",
    }
}

fn confidence(classification: &FailureClassification, has_head_evidence: bool) -> &'static str {
    if !has_head_evidence {
        return "low";
    }

    match classification {
        FailureClassification::MasterRed | FailureClassification::CodeRegression => "high",
        FailureClassification::StaleBase | FailureClassification::InfraFailure => "medium",
        FailureClassification::Flaky | FailureClassification::Unknown => "low",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use color_eyre::eyre::bail;

    fn fixture(name: &str) -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("tests")
            .join("fixtures")
            .join("failure-classifier")
            .join(name)
    }

    #[test]
    fn fixture_master_red_classifies_master_red() -> Result<()> {
        let input = load_input(&fixture("master-red.json"))?;
        let receipt = classify(&input);
        if receipt.classification != FailureClassification::MasterRed {
            bail!("expected MASTER_RED, got {:?}", receipt.classification);
        }
        Ok(())
    }

    #[test]
    fn fixture_stale_base_classifies_stale_base() -> Result<()> {
        let input = load_input(&fixture("stale-base.json"))?;
        let receipt = classify(&input);
        if receipt.classification != FailureClassification::StaleBase {
            bail!("expected STALE_BASE, got {:?}", receipt.classification);
        }
        Ok(())
    }

    #[test]
    fn fixture_pr_owned_classifies_pr_owned() -> Result<()> {
        let input = load_input(&fixture("pr-owned.json"))?;
        let receipt = classify(&input);
        if receipt.classification != FailureClassification::CodeRegression {
            bail!("expected CODE_REGRESSION (PR_OWNED), got {:?}", receipt.classification);
        }
        Ok(())
    }

    #[test]
    fn fixture_missing_data_classifies_unknown() -> Result<()> {
        let input = load_input(&fixture("unknown.json"))?;
        let receipt = classify(&input);
        if receipt.classification != FailureClassification::Unknown {
            bail!("expected UNKNOWN, got {:?}", receipt.classification);
        }
        Ok(())
    }

    /// Verify the receipt shape is schema-conformant: required fields present,
    /// check value matches schema const, classification is snake_case.
    #[test]
    fn receipt_fields_are_schema_conformant() -> Result<()> {
        let input = load_input(&fixture("pr-owned.json"))?;
        let receipt = classify(&input);

        // check field must match schema const "failure-classifier"
        assert_eq!(receipt.check, "failure-classifier", "check must match schema const");

        // schema_version must be non-empty
        assert!(!receipt.schema_version.is_empty(), "schema_version must not be empty");

        // event must be a known value
        assert!(
            matches!(receipt.event.as_str(), "pull_request" | "merge_group" | "push" | "local"),
            "event must be a known value, got: {}",
            receipt.event
        );

        // verdict for code_regression must be "fail"
        assert_eq!(receipt.verdict, "fail", "code_regression receipt must have verdict=fail");

        // classification serializes as snake_case
        let json = serde_json::to_string(&receipt)?;
        assert!(
            json.contains("\"code_regression\""),
            "classification must serialize as snake_case 'code_regression', got: {json}"
        );
        assert!(
            !json.contains("\"PR_OWNED\"") && !json.contains("\"CodeRegression\""),
            "classification must NOT serialize as SCREAMING_SNAKE_CASE or PascalCase"
        );

        Ok(())
    }

    /// Verify that non-PR-owned classifications produce verdict="pass".
    #[test]
    fn receipt_verdict_pass_for_master_red() -> Result<()> {
        let input = load_input(&fixture("master-red.json"))?;
        let receipt = classify(&input);
        assert_eq!(receipt.verdict, "pass", "master_red is not a PR-owned failure; verdict must be pass");
        Ok(())
    }

    /// When master_checks is absent but behind_master=true, the classifier must
    /// NOT classify as STALE_BASE — it cannot confirm master is green without
    /// master evidence. Conservative fallback is UNKNOWN.
    #[test]
    fn behind_master_without_master_checks_is_unknown_not_stale_base() {
        let input = SnapshotInput {
            pr: PullRequestInput {
                number: Some(9999),
                head_sha: Some("head-sha-999".to_string()),
                master_sha: Some("master-sha-abc".to_string()),
                behind_master: true,
                changed_files: vec!["src/main.rs".to_string()],
            },
            pr_checks: vec![CheckInput {
                name: Some("ci / test".to_string()),
                signature: Some("ci / test: build failure".to_string()),
                sha: Some("head-sha-999".to_string()),
                conclusion: Some("failure".to_string()),
                ..CheckInput::default()
            }],
            master_checks: vec![], // no master data
            ..SnapshotInput::default()
        };

        let receipt = classify(&input);
        assert_ne!(
            receipt.classification,
            FailureClassification::StaleBase,
            "should not classify as STALE_BASE without master evidence to confirm master is green"
        );
        // Without file overlap and no infra pattern, falls to UNKNOWN
        assert_eq!(
            receipt.classification,
            FailureClassification::Unknown,
            "insufficient evidence should produce UNKNOWN"
        );
    }

    /// When head_sha is None, any failing check is treated as head-evidence.
    /// This is a known conservative behavior: classifier trusts available data.
    #[test]
    fn no_head_sha_includes_all_failing_checks() {
        let input = SnapshotInput {
            pr: PullRequestInput {
                number: Some(8888),
                head_sha: None, // no head SHA in input
                master_sha: None,
                behind_master: false,
                changed_files: vec!["crates/foo/src/lib.rs".to_string()],
            },
            pr_checks: vec![CheckInput {
                name: Some("ci / test".to_string()),
                signature: Some("ci / test: link error".to_string()),
                sha: Some("some-old-sha".to_string()),
                conclusion: Some("failure".to_string()),
                files: vec!["crates/foo/src/lib.rs".to_string()],
                ..CheckInput::default()
            }],
            master_checks: vec![],
            ..SnapshotInput::default()
        };

        let receipt = classify(&input);
        // With no head_sha, the check passes the SHA filter and produces evidence.
        // File overlap exists, so should classify as CodeRegression.
        assert_eq!(
            receipt.classification,
            FailureClassification::CodeRegression,
            "without head_sha, failing check with file overlap should classify as code_regression"
        );
    }
}
