use std::fs;
use std::path::PathBuf;

use color_eyre::eyre::{Context, ContextCompat, Result};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone)]
pub struct FailureClassifierConfig {
    pub snapshot: Option<PathBuf>,
    pub fixture: Option<PathBuf>,
    pub receipt: Option<PathBuf>,
}

#[derive(Debug, Deserialize)]
struct FailureClassifierInput {
    current_head_sha: String,
    gate: Option<String>,
    pr: RunStatus,
    master: Option<RunStatus>,
    merge_group: Option<RunStatus>,
    #[serde(default)]
    known_infra_signatures: Vec<String>,
    #[serde(default)]
    receipt_artifacts: Vec<ReceiptArtifact>,
    #[serde(default)]
    affected_prs: Vec<String>,
    #[serde(default)]
    base_behind_master: bool,
    #[serde(default)]
    flaky_indicators: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct RunStatus {
    sha: Option<String>,
    status: JobStatus,
    signature: Option<String>,
    #[serde(default)]
    observed_head_sha: Option<String>,
    #[serde(default)]
    changed_files: Vec<String>,
    #[serde(default)]
    failed_files: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct ReceiptArtifact {
    signature: Option<String>,
}

#[derive(Debug, Clone, Copy, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
enum JobStatus {
    Success,
    Failure,
    Pending,
    Missing,
}

#[derive(Debug, Serialize, PartialEq, Eq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
enum Classification {
    PrOwned,
    StaleBase,
    MasterRed,
    InfraFailure,
    Flaky,
    Unknown,
}

#[derive(Debug, Serialize)]
struct FailureClassifierReceipt {
    check: String,
    signature: String,
    affected_prs: Vec<String>,
    master_sha: Option<String>,
    master_same_signature: bool,
    classification: Classification,
    recommended_action: String,
    confidence: f64,
    evidence: Vec<String>,
}

pub fn run(config: FailureClassifierConfig) -> Result<()> {
    let input_path = config.fixture.or(config.snapshot).context(
        "provide either --snapshot <path> (queue payload) or --fixture <path> (test payload)",
    )?;

    let raw = fs::read_to_string(&input_path)
        .with_context(|| format!("reading failure-classifier input: {}", input_path.display()))?;
    let input: FailureClassifierInput = serde_json::from_str(&raw)
        .with_context(|| format!("parsing failure-classifier input: {}", input_path.display()))?;

    let receipt = classify(input);
    let receipt_json = serde_json::to_string_pretty(&receipt)
        .context("serializing failure-classifier receipt JSON")?;

    if let Some(path) = config.receipt {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)
                .with_context(|| format!("creating receipt directory: {}", parent.display()))?;
        }
        fs::write(&path, format!("{receipt_json}\n"))
            .with_context(|| format!("writing failure-classifier receipt: {}", path.display()))?;
    }

    println!("{receipt_json}");
    Ok(())
}

fn classify(input: FailureClassifierInput) -> FailureClassifierReceipt {
    let signature = derive_signature(&input);
    let mut evidence = vec![format!("current_head_sha={}", input.current_head_sha)];
    if let Some(gate) = &input.gate {
        evidence.push(format!("gate={gate}"));
    }

    let master_same_signature = input.master.as_ref().is_some_and(|master| {
        master.status == JobStatus::Failure
            && master
                .signature
                .as_ref()
                .is_some_and(|master_signature| master_signature == &signature)
    });

    if input.pr.status != JobStatus::Failure {
        evidence.push("pr status is not failure".to_string());
        return finalize(
            signature,
            input.affected_prs,
            input.master.and_then(|m| m.sha),
            master_same_signature,
            Classification::Unknown,
            0.25,
            evidence,
        );
    }

    if input.known_infra_signatures.iter().any(|known_signature| known_signature == &signature) {
        evidence.push("signature matched known infra signature".to_string());
        return finalize(
            signature,
            input.affected_prs,
            input.master.and_then(|m| m.sha),
            master_same_signature,
            Classification::InfraFailure,
            0.98,
            evidence,
        );
    }

    if master_same_signature {
        evidence.push("master failed on same gate/signature".to_string());
        return finalize(
            signature,
            input.affected_prs,
            input.master.and_then(|m| m.sha),
            master_same_signature,
            Classification::MasterRed,
            0.97,
            evidence,
        );
    }

    if input.base_behind_master
        && input.master.as_ref().is_some_and(|m| m.status == JobStatus::Success)
    {
        evidence.push("PR base is behind green master".to_string());
        return finalize(
            signature,
            input.affected_prs,
            input.master.and_then(|m| m.sha),
            master_same_signature,
            Classification::StaleBase,
            0.9,
            evidence,
        );
    }

    if !input.flaky_indicators.is_empty() {
        evidence.push(format!("flaky indicators: {}", input.flaky_indicators.join(",")));
        return finalize(
            signature,
            input.affected_prs,
            input.master.and_then(|m| m.sha),
            master_same_signature,
            Classification::Flaky,
            0.7,
            evidence,
        );
    }

    if let Some(merge_group) = &input.merge_group {
        evidence.push(format!("merge_group status={}", as_str(merge_group.status)));
    }

    if is_pr_owned(&input, &mut evidence) {
        return finalize(
            signature,
            input.affected_prs,
            input.master.and_then(|m| m.sha),
            master_same_signature,
            Classification::PrOwned,
            0.92,
            evidence,
        );
    }

    evidence.push("insufficient evidence for PR ownership".to_string());
    finalize(
        signature,
        input.affected_prs,
        input.master.and_then(|m| m.sha),
        master_same_signature,
        Classification::Unknown,
        0.35,
        evidence,
    )
}

fn derive_signature(input: &FailureClassifierInput) -> String {
    if let Some(signature) = &input.pr.signature {
        return signature.clone();
    }

    for artifact in &input.receipt_artifacts {
        if let Some(signature) = &artifact.signature {
            return signature.clone();
        }
    }

    "unknown-signature".to_string()
}

fn is_pr_owned(input: &FailureClassifierInput, evidence: &mut Vec<String>) -> bool {
    let Some(observed_head_sha) = &input.pr.observed_head_sha else {
        evidence.push("missing pr.observed_head_sha".to_string());
        return false;
    };

    if observed_head_sha != &input.current_head_sha {
        evidence.push(format!(
            "head mismatch: observed={} current={}",
            observed_head_sha, input.current_head_sha
        ));
        return false;
    }

    let changed_matches_failure = input.pr.failed_files.iter().any(|failed_file| {
        input.pr.changed_files.iter().any(|changed_file| changed_file == failed_file)
    });

    if !changed_matches_failure {
        evidence.push("failed files do not overlap changed files".to_string());
        return false;
    }

    evidence.push(
        "current head matches failing run and failed files overlap changed files".to_string(),
    );
    true
}

fn finalize(
    signature: String,
    affected_prs: Vec<String>,
    master_sha: Option<String>,
    master_same_signature: bool,
    classification: Classification,
    confidence: f64,
    evidence: Vec<String>,
) -> FailureClassifierReceipt {
    FailureClassifierReceipt {
        check: "Failure Classifier".to_string(),
        signature,
        affected_prs,
        master_sha,
        master_same_signature,
        recommended_action: recommended_action(&classification).to_string(),
        classification,
        confidence,
        evidence,
    }
}

fn recommended_action(classification: &Classification) -> &'static str {
    match classification {
        Classification::PrOwned => "NEEDS_CI_FIX / builder",
        Classification::StaleBase => "NEEDS_CASCADE_UPDATE",
        Classification::MasterRed => "master incident / no PR-owned label",
        Classification::InfraFailure => "infra/tooling route",
        Classification::Flaky => "rerun/observe",
        Classification::Unknown => "human classification",
    }
}

fn as_str(status: JobStatus) -> &'static str {
    match status {
        JobStatus::Success => "success",
        JobStatus::Failure => "failure",
        JobStatus::Pending => "pending",
        JobStatus::Missing => "missing",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn load_fixture(name: &str) -> Result<FailureClassifierInput> {
        let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures/failure-classifier")
            .join(name);
        let raw = fs::read_to_string(&path)
            .with_context(|| format!("reading fixture file: {}", path.display()))?;
        let input = serde_json::from_str(&raw)
            .with_context(|| format!("parsing fixture file: {}", path.display()))?;
        Ok(input)
    }

    #[test]
    fn fixture_master_red_classifies_correctly() -> Result<()> {
        let receipt = classify(load_fixture("master-red.json")?);
        assert_eq!(receipt.classification, Classification::MasterRed);
        Ok(())
    }

    #[test]
    fn fixture_stale_base_classifies_correctly() -> Result<()> {
        let receipt = classify(load_fixture("stale-base.json")?);
        assert_eq!(receipt.classification, Classification::StaleBase);
        Ok(())
    }

    #[test]
    fn fixture_pr_owned_classifies_correctly() -> Result<()> {
        let receipt = classify(load_fixture("pr-owned.json")?);
        assert_eq!(receipt.classification, Classification::PrOwned);
        Ok(())
    }

    #[test]
    fn fixture_unknown_classifies_correctly() -> Result<()> {
        let receipt = classify(load_fixture("unknown-missing-data.json")?);
        assert_eq!(receipt.classification, Classification::Unknown);
        Ok(())
    }
}
