use std::fs;
use std::path::{Path, PathBuf};

use color_eyre::eyre::{Context, Result, bail};
use serde::{Deserialize, Serialize};
use serde_json::Value;

const CHECK_NAME: &str = "Failure Classifier";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum Classification {
    PrOwned,
    StaleBase,
    MasterRed,
    InfraFailure,
    Flaky,
    Unknown,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct FailureClassifierReceipt {
    pub check: String,
    pub signature: String,
    pub affected_prs: Vec<u64>,
    pub master_sha: Option<String>,
    pub master_same_signature: bool,
    pub classification: Classification,
    pub recommended_action: String,
    pub confidence: f64,
    pub evidence: Vec<String>,
}

#[derive(Debug, Clone, Deserialize, Default)]
struct InputEnvelope {
    #[serde(default)]
    pr_number: Option<u64>,
    #[serde(default)]
    affected_prs: Vec<u64>,
    #[serde(default)]
    pr_head_sha: Option<String>,
    #[serde(default)]
    pr_status: Status,
    #[serde(default)]
    master_status: Option<Status>,
    #[serde(default)]
    merge_group_status: Option<Status>,
    #[serde(default)]
    infra_signatures: Vec<String>,
    #[serde(default)]
    receipt_artifacts: Vec<Value>,
    #[serde(default)]
    pr_is_behind_master: Option<bool>,
    #[serde(default)]
    current_head_evidence: Option<bool>,
    #[serde(default)]
    flaky_signal: Option<bool>,
    #[serde(default)]
    pr_touches_failing_diff: Option<bool>,
}

#[derive(Debug, Clone, Deserialize, Default)]
struct Status {
    #[serde(default)]
    sha: Option<String>,
    #[serde(default)]
    state: Option<String>,
    #[serde(default)]
    signature: Option<String>,
}

#[derive(Debug, Clone)]
pub struct Config {
    pub snapshot: Option<PathBuf>,
    pub receipt: Option<PathBuf>,
    pub fixture: Option<PathBuf>,
}

pub fn run(config: Config) -> Result<()> {
    if config.snapshot.is_some() && config.fixture.is_some() {
        bail!("--snapshot and --fixture are mutually exclusive");
    }

    let input_path = config
        .fixture
        .as_ref()
        .or(config.snapshot.as_ref())
        .ok_or_else(|| color_eyre::eyre::eyre!("must provide --snapshot or --fixture"))?;

    let input = parse_input(input_path)?;
    let receipt = classify(input);

    let rendered = serde_json::to_string_pretty(&receipt).context("serializing receipt")?;
    println!("{rendered}");

    if let Some(receipt_path) = config.receipt {
        if let Some(parent) = receipt_path.parent() {
            fs::create_dir_all(parent).with_context(|| format!("creating {}", parent.display()))?;
        }
        fs::write(&receipt_path, format!("{rendered}\n"))
            .with_context(|| format!("writing {}", receipt_path.display()))?;
    }

    Ok(())
}

fn parse_input(path: &Path) -> Result<InputEnvelope> {
    let raw = fs::read_to_string(path).with_context(|| format!("reading {}", path.display()))?;
    let value: Value =
        serde_json::from_str(&raw).with_context(|| format!("parsing {}", path.display()))?;

    if value.is_object() {
        return serde_json::from_value(value).context("decoding classifier input object");
    }

    if let Some(items) = value.as_array()
        && let Some(first) = items.first()
    {
        return serde_json::from_value(first.clone()).context("decoding first fixture entry");
    }

    bail!("unsupported failure classifier input shape")
}

fn classify(input: InputEnvelope) -> FailureClassifierReceipt {
    let mut evidence = Vec::new();
    if let Some(pr_head_sha) = input.pr_head_sha.as_deref() {
        evidence.push(format!("evaluated current PR head SHA {pr_head_sha}"));
    }
    let signature = signature_for(&input);
    let master_signature = input.master_status.as_ref().and_then(|status| status.signature.clone());
    let master_sha = input.master_status.as_ref().and_then(|status| status.sha.clone());
    let master_same_signature = master_signature.as_deref() == Some(signature.as_str());

    let affected_prs = affected_prs(&input);

    let (classification, recommended_action, confidence) = if is_infra_failure(&input, &signature) {
        evidence.push("failure signature matches known infrastructure signature".to_string());
        (Classification::InfraFailure, "ROUTE_INFRA_TOOLING".to_string(), 0.95)
    } else if master_same_signature && is_failure(input.master_status.as_ref()) {
        evidence.push("master is red with same gate signature".to_string());
        (Classification::MasterRed, "OPEN_MASTER_INCIDENT".to_string(), 0.93)
    } else if input.pr_is_behind_master.unwrap_or(false)
        && is_success(input.master_status.as_ref())
        && is_failure(Some(&input.pr_status))
    {
        evidence.push("PR head is behind a green master for the same gate".to_string());
        (Classification::StaleBase, "NEEDS_CASCADE_UPDATE".to_string(), 0.87)
    } else if input.flaky_signal.unwrap_or(false)
        || (is_failure(input.merge_group_status.as_ref()) && is_success(Some(&input.pr_status)))
    {
        evidence.push("inconsistent outcomes suggest a flaky failure".to_string());
        (Classification::Flaky, "RERUN_AND_OBSERVE".to_string(), 0.78)
    } else if input.current_head_evidence.unwrap_or(false)
        && input.pr_touches_failing_diff.unwrap_or(false)
        && is_failure(Some(&input.pr_status))
    {
        evidence.push("current-head failing evidence points at changed PR diff".to_string());
        (Classification::PrOwned, "NEEDS_CI_FIX / builder".to_string(), 0.89)
    } else {
        evidence.push("insufficient correlated evidence for deterministic routing".to_string());
        (Classification::Unknown, "HUMAN_CLASSIFICATION".to_string(), 0.4)
    };

    if input.current_head_evidence != Some(true) && classification == Classification::PrOwned {
        evidence.push("PR_OWNED requires current-head evidence; downgraded to UNKNOWN".to_string());
        return FailureClassifierReceipt {
            check: CHECK_NAME.to_string(),
            signature,
            affected_prs,
            master_sha,
            master_same_signature,
            classification: Classification::Unknown,
            recommended_action: "HUMAN_CLASSIFICATION".to_string(),
            confidence: 0.35,
            evidence,
        };
    }

    FailureClassifierReceipt {
        check: CHECK_NAME.to_string(),
        signature,
        affected_prs,
        master_sha,
        master_same_signature,
        classification,
        recommended_action,
        confidence,
        evidence,
    }
}

fn affected_prs(input: &InputEnvelope) -> Vec<u64> {
    if !input.affected_prs.is_empty() {
        return input.affected_prs.clone();
    }
    input.pr_number.into_iter().collect()
}

fn signature_for(input: &InputEnvelope) -> String {
    input
        .pr_status
        .signature
        .clone()
        .or_else(|| input.merge_group_status.as_ref().and_then(|s| s.signature.clone()))
        .or_else(|| signature_from_artifact(&input.receipt_artifacts))
        .unwrap_or_else(|| "unknown-signature".to_string())
}

fn signature_from_artifact(artifacts: &[Value]) -> Option<String> {
    artifacts.iter().find_map(|artifact| {
        artifact.get("signature").and_then(Value::as_str).map(ToString::to_string)
    })
}

fn is_infra_failure(input: &InputEnvelope, signature: &str) -> bool {
    input.infra_signatures.iter().any(|infra_sig| infra_sig.eq_ignore_ascii_case(signature))
}

fn is_failure(status: Option<&Status>) -> bool {
    matches!(
        status.and_then(|state| state.state.as_deref()),
        Some("failure") | Some("failed") | Some("error")
    )
}

fn is_success(status: Option<&Status>) -> bool {
    matches!(status.and_then(|state| state.state.as_deref()), Some("success") | Some("passed"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn master_red_wins_when_same_signature_fails() {
        let input = InputEnvelope {
            pr_status: Status {
                state: Some("failure".to_string()),
                signature: Some("gate::pr-fast::test".to_string()),
                sha: None,
            },
            master_status: Some(Status {
                state: Some("failure".to_string()),
                signature: Some("gate::pr-fast::test".to_string()),
                sha: Some("abc123".to_string()),
            }),
            ..InputEnvelope::default()
        };

        let receipt = classify(input);
        assert_eq!(receipt.classification, Classification::MasterRed);
    }

    #[test]
    fn never_marks_pr_owned_without_current_head_evidence() {
        let input = InputEnvelope {
            pr_status: Status {
                state: Some("failure".to_string()),
                signature: Some("gate::pr-fast::lint".to_string()),
                sha: None,
            },
            current_head_evidence: Some(false),
            pr_touches_failing_diff: Some(true),
            ..InputEnvelope::default()
        };

        let receipt = classify(input);
        assert_eq!(receipt.classification, Classification::Unknown);
    }
}
