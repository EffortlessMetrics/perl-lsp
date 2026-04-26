use std::fs;
use std::path::{Path, PathBuf};

use color_eyre::eyre::{Context, Result};
use serde::{Deserialize, Serialize};

const DEFAULT_RECEIPT_PATH: &str = "target/receipts/failure-classifier.json";
const CHECK_NAME: &str = "Failure Classifier";
const INFRA_SIGNATURES: &[&str] = &[
    "timed out waiting for",
    "network is unreachable",
    "temporary failure in name resolution",
    "service unavailable",
    "runner lost communication",
    "no space left on device",
    "resource temporarily unavailable",
    "connection reset by peer",
    "502 bad gateway",
    "503 service unavailable",
    "github actions service",
];
const FLAKY_SIGNATURES: &[&str] = &[
    "test timed out",
    "intermittent",
    "flake",
    "segmentation fault",
    "signal: 11",
    "assertion failed",
];

#[derive(Debug, Clone)]
pub struct FailureClassifierConfig {
    pub snapshot: Option<PathBuf>,
    pub fixture: Option<PathBuf>,
    pub receipt: Option<PathBuf>,
}

#[derive(Debug, Clone, Deserialize)]
struct FailureInput {
    pr_number: Option<u64>,
    pr_head_sha: Option<String>,
    pr_head_is_behind_master: Option<bool>,
    failed_prs: Option<Vec<u64>>,
    signature: Option<String>,
    pr_status: Option<GateStatus>,
    master_status: Option<GateStatus>,
    merge_group_status: Option<GateStatus>,
    known_infra_signatures: Option<Vec<String>>,
    receipt_artifacts: Option<Vec<ReceiptArtifact>>,
}

#[derive(Debug, Clone, Deserialize)]
struct GateStatus {
    sha: Option<String>,
    gate: Option<String>,
    state: Option<GateState>,
    signature: Option<String>,
    failing_files: Option<Vec<String>>,
}

#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
enum GateState {
    Success,
    Failure,
    Pending,
    Missing,
}

#[derive(Debug, Clone, Deserialize)]
struct ReceiptArtifact {
    signature: Option<String>,
    flaky: Option<bool>,
    infra: Option<bool>,
    evidence: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct FailureReceipt {
    check: String,
    signature: String,
    affected_prs: Vec<u64>,
    master_sha: Option<String>,
    master_same_signature: bool,
    classification: Classification,
    recommended_action: String,
    confidence: f64,
    evidence: Vec<String>,
}

#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
enum Classification {
    PrOwned,
    StaleBase,
    MasterRed,
    InfraFailure,
    Flaky,
    Unknown,
}

pub fn run(config: FailureClassifierConfig) -> Result<()> {
    let source = config
        .fixture
        .as_ref()
        .or(config.snapshot.as_ref())
        .ok_or_else(|| color_eyre::eyre::eyre!("pass --fixture <file> or --snapshot <file>"))?;
    let input = read_input(source)?;
    let receipt = classify(&input);
    let receipt_json = serde_json::to_string_pretty(&receipt)?;

    if config.snapshot.is_some() {
        let out_path = config.receipt.unwrap_or_else(|| PathBuf::from(DEFAULT_RECEIPT_PATH));
        if let Some(parent) = out_path.parent() {
            fs::create_dir_all(parent).with_context(|| format!("creating {}", parent.display()))?;
        }
        fs::write(&out_path, format!("{receipt_json}\n"))
            .with_context(|| format!("writing {}", out_path.display()))?;
        println!("Wrote failure-classifier receipt to {}", out_path.display());
    } else {
        println!("{receipt_json}");
    }

    Ok(())
}

fn read_input(path: &Path) -> Result<FailureInput> {
    let raw = fs::read_to_string(path).with_context(|| format!("reading {}", path.display()))?;
    serde_json::from_str(&raw).with_context(|| format!("parsing {}", path.display()))
}

fn classify(input: &FailureInput) -> FailureReceipt {
    let signature = [
        input.signature.clone(),
        input.pr_status.as_ref().and_then(|status| status.signature.clone()),
        input.receipt_artifacts.as_ref().and_then(|artifacts| {
            artifacts.first().and_then(|artifact| artifact.signature.clone())
        }),
    ]
    .into_iter()
    .flatten()
    .map(|candidate| candidate.trim().to_string())
    .find(|candidate| !candidate.is_empty())
    .unwrap_or_else(|| "unknown-signature".to_string());

    let evidence = collect_evidence(input, &signature);
    let affected_prs =
        input.failed_prs.clone().or_else(|| input.pr_number.map(|pr| vec![pr])).unwrap_or_default();

    let master_same_signature = input.master_status.as_ref().is_some_and(|status| {
        is_failure(status) && status.signature.as_deref() == Some(signature.as_str())
    });

    let (classification, recommended_action, confidence) = if is_infra_failure(input, &signature) {
        (Classification::InfraFailure, "Route to infra/tooling triage queue".to_string(), 0.95)
    } else if is_flaky(input) {
        (Classification::Flaky, "Rerun failing gate and observe recurrence".to_string(), 0.65)
    } else if master_same_signature {
        (
            Classification::MasterRed,
            "Open/attach to master incident; avoid PR-owned labels".to_string(),
            0.95,
        )
    } else if is_stale_base(input) {
        (
            Classification::StaleBase,
            "Request cascade/base update before CI-fix routing".to_string(),
            0.9,
        )
    } else if is_pr_owned(input) {
        (Classification::PrOwned, "Route to NEEDS_CI_FIX / builder".to_string(), 0.9)
    } else {
        (Classification::Unknown, "Escalate for human classification".to_string(), 0.4)
    };

    FailureReceipt {
        check: CHECK_NAME.to_string(),
        signature,
        affected_prs,
        master_sha: input.master_status.as_ref().and_then(|status| status.sha.clone()),
        master_same_signature,
        classification,
        recommended_action,
        confidence,
        evidence,
    }
}

fn collect_evidence(input: &FailureInput, signature: &str) -> Vec<String> {
    let mut evidence = Vec::new();
    evidence.push(format!("signature={signature}"));

    if let Some(status) = &input.pr_status {
        if let Some(gate) = &status.gate {
            evidence.push(format!("pr_gate={gate}"));
        }
        if let Some(state) = status.state {
            evidence.push(format!("pr_state={state:?}"));
        }
        if let Some(files) = &status.failing_files {
            evidence.push(format!("pr_failing_files={}", files.join(",")));
        }
    }

    if let Some(master) = &input.master_status {
        if let Some(state) = master.state {
            evidence.push(format!("master_state={state:?}"));
        }
        if let Some(master_signature) = &master.signature {
            evidence.push(format!("master_signature={master_signature}"));
        }
    }

    if let Some(merge_group) = &input.merge_group_status {
        if let Some(state) = merge_group.state {
            evidence.push(format!("merge_group_state={state:?}"));
        }
    }

    if let Some(artifacts) = &input.receipt_artifacts {
        for artifact in artifacts {
            if let Some(text) = &artifact.evidence {
                evidence.push(format!("artifact={text}"));
            }
        }
    }

    evidence
}

fn is_failure(status: &GateStatus) -> bool {
    matches!(status.state, Some(GateState::Failure))
}

fn is_infra_failure(input: &FailureInput, signature: &str) -> bool {
    let sig = signature.to_ascii_lowercase();
    if INFRA_SIGNATURES.iter().any(|needle| sig.contains(needle)) {
        return true;
    }

    if let Some(extra_signatures) = &input.known_infra_signatures {
        let infra_match = extra_signatures
            .iter()
            .map(|entry| entry.to_ascii_lowercase())
            .any(|needle| sig.contains(&needle));
        if infra_match {
            return true;
        }
    }

    input
        .receipt_artifacts
        .as_ref()
        .is_some_and(|artifacts| artifacts.iter().any(|artifact| artifact.infra == Some(true)))
}

fn is_flaky(input: &FailureInput) -> bool {
    if input
        .receipt_artifacts
        .as_ref()
        .is_some_and(|artifacts| artifacts.iter().any(|artifact| artifact.flaky == Some(true)))
    {
        return true;
    }

    let signature = input
        .signature
        .as_deref()
        .or_else(|| input.pr_status.as_ref().and_then(|status| status.signature.as_deref()))
        .unwrap_or_default()
        .to_ascii_lowercase();

    FLAKY_SIGNATURES.iter().any(|needle| signature.contains(needle))
}

fn is_stale_base(input: &FailureInput) -> bool {
    let Some(pr_status) = input.pr_status.as_ref() else {
        return false;
    };
    let Some(master_status) = input.master_status.as_ref() else {
        return false;
    };

    matches!(pr_status.state, Some(GateState::Failure))
        && matches!(master_status.state, Some(GateState::Success))
        && input.pr_head_is_behind_master == Some(true)
}

fn is_pr_owned(input: &FailureInput) -> bool {
    let Some(pr_status) = input.pr_status.as_ref() else {
        return false;
    };
    let Some(master_status) = input.master_status.as_ref() else {
        return false;
    };

    matches!(pr_status.state, Some(GateState::Failure))
        && matches!(master_status.state, Some(GateState::Success))
        && input.pr_head_sha.is_some()
        && pr_status.failing_files.as_ref().is_some_and(|files| !files.is_empty())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn load_fixture(path: &str) -> Result<FailureInput> {
        let full_path = Path::new(env!("CARGO_MANIFEST_DIR")).join(path);
        read_input(&full_path)
    }

    #[test]
    fn fixture_master_red_classifies_master_red() -> Result<()> {
        let input = load_fixture("tests/fixtures/failure-classifier/master-red.json")?;
        let receipt = classify(&input);

        assert_eq!(receipt.classification, Classification::MasterRed);
        Ok(())
    }

    #[test]
    fn fixture_stale_base_classifies_stale_base() -> Result<()> {
        let input = load_fixture("tests/fixtures/failure-classifier/stale-base.json")?;
        let receipt = classify(&input);

        assert_eq!(receipt.classification, Classification::StaleBase);
        Ok(())
    }

    #[test]
    fn fixture_pr_owned_classifies_pr_owned() -> Result<()> {
        let input = load_fixture("tests/fixtures/failure-classifier/pr-owned.json")?;
        let receipt = classify(&input);

        assert_eq!(receipt.classification, Classification::PrOwned);
        Ok(())
    }

    #[test]
    fn fixture_missing_data_classifies_unknown() -> Result<()> {
        let input = load_fixture("tests/fixtures/failure-classifier/unknown.json")?;
        let receipt = classify(&input);

        assert_eq!(receipt.classification, Classification::Unknown);
        Ok(())
    }

    #[test]
    fn requires_snapshot_or_fixture() -> Result<()> {
        let run_result =
            run(FailureClassifierConfig { snapshot: None, fixture: None, receipt: None });
        let err = match run_result {
            Ok(()) => {
                return Err(color_eyre::eyre::eyre!("expected error when no input flag provided"));
            }
            Err(err) => err,
        };
        let msg = err.to_string();
        if !msg.contains("--fixture") {
            return Err(color_eyre::eyre::eyre!("missing usage message in error: {msg}"));
        }
        Ok(())
    }
}
