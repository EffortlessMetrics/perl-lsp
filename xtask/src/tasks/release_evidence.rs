use color_eyre::eyre::{Context, ContextCompat, Result, bail};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use crate::utils::project_root;

const POLICY_PATH: &str = ".ci/release/evidence.toml";
const DEFAULT_SUMMARY_RECEIPT: &str = "target/receipts/release-evidence.json";

#[derive(Debug, Deserialize)]
struct EvidencePolicyFile {
    release_evidence: Option<ReleaseEvidencePolicy>,
    advisory: Option<AdvisoryPolicy>,
}

#[derive(Debug, Deserialize)]
struct ReleaseEvidencePolicy {
    required: Vec<String>,
    summary_receipt: Option<String>,
}

#[derive(Debug, Deserialize)]
struct AdvisoryPolicy {
    receipt: Option<String>,
    release_blocking: Option<bool>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ReleaseEvidenceSummary {
    version: String,
    bundle_path: String,
    generated_at_utc: String,
    status: EvidenceStatus,
    checks: Vec<EvidenceCheck>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum EvidenceStatus {
    Pass,
    Warning,
    Fail,
}

#[derive(Debug, Serialize, Deserialize)]
struct EvidenceCheck {
    name: String,
    path: String,
    required: bool,
    status: EvidenceStatus,
    detail: String,
}

#[derive(Debug, Deserialize)]
struct GenericReceipt {
    status: Option<String>,
}

#[derive(Debug, Deserialize)]
struct AdvisoryReceipt {
    status: Option<String>,
    release_blocking: Option<bool>,
}

pub fn run_evidence(version: String, out: PathBuf) -> Result<()> {
    let root = project_root()?;
    let policy = load_policy(&root)?;
    let summary = evaluate_bundle(&version, &out, &policy)?;

    let summary_receipt = root.join(policy.summary_receipt);
    write_summary(&summary_receipt, &summary)?;
    println!("Release evidence summary written to {}", summary_receipt.display());

    match summary.status {
        EvidenceStatus::Pass | EvidenceStatus::Warning => Ok(()),
        EvidenceStatus::Fail => bail!("release evidence check failed"),
    }
}

pub fn run_verify_evidence(version: String, receipt: PathBuf) -> Result<()> {
    let root = project_root()?;
    let summary_path = if receipt.is_absolute() { receipt } else { root.join(receipt) };

    let raw = fs::read_to_string(&summary_path)
        .with_context(|| format!("failed to read summary receipt {}", summary_path.display()))?;
    let summary: ReleaseEvidenceSummary =
        serde_json::from_str(&raw).context("failed to parse release evidence summary JSON")?;

    if summary.version != version {
        bail!("release evidence version mismatch: expected {}, got {}", version, summary.version);
    }

    match summary.status {
        EvidenceStatus::Pass => Ok(()),
        EvidenceStatus::Warning => {
            println!("release evidence verified with advisory warnings");
            Ok(())
        }
        EvidenceStatus::Fail => bail!("release evidence receipt reports failure"),
    }
}

fn evaluate_bundle(
    version: &str,
    out: &Path,
    policy: &ResolvedPolicy,
) -> Result<ReleaseEvidenceSummary> {
    let mut checks = Vec::new();
    let mut has_fail = false;
    let mut has_warning = false;

    for required in &policy.required_receipts {
        let receipt_path = out.join(required);
        if !receipt_path.exists() {
            has_fail = true;
            checks.push(EvidenceCheck {
                name: required.clone(),
                path: receipt_path.display().to_string(),
                required: true,
                status: EvidenceStatus::Fail,
                detail: "missing required receipt".to_string(),
            });
            continue;
        }

        let status = read_receipt_status(&receipt_path)?;
        let mapped = map_status(&status);
        if mapped == EvidenceStatus::Fail {
            has_fail = true;
        }

        checks.push(EvidenceCheck {
            name: required.clone(),
            path: receipt_path.display().to_string(),
            required: true,
            status: mapped,
            detail: format!("receipt status={status}"),
        });
    }

    let advisory_file = out.join(&policy.advisory_receipt);
    if advisory_file.exists() {
        let advisory_raw = fs::read_to_string(&advisory_file).with_context(|| {
            format!("failed to read advisory receipt {}", advisory_file.display())
        })?;
        let advisory: AdvisoryReceipt =
            serde_json::from_str(&advisory_raw).context("failed to parse advisory receipt")?;
        let status_text = advisory.status.unwrap_or_else(|| "unknown".to_string());
        let mapped = map_status(&status_text);
        let is_release_blocking =
            advisory.release_blocking.unwrap_or(policy.advisory_release_blocking);

        let (status, detail) = if mapped == EvidenceStatus::Fail && !is_release_blocking {
            has_warning = true;
            (
                EvidenceStatus::Warning,
                "advisory failure classified as warning (non-release-blocking policy)".to_string(),
            )
        } else {
            if mapped == EvidenceStatus::Fail {
                has_fail = true;
            }
            (
                mapped,
                format!("advisory status={status_text}, release_blocking={is_release_blocking}"),
            )
        };

        checks.push(EvidenceCheck {
            name: policy.advisory_receipt.clone(),
            path: advisory_file.display().to_string(),
            required: true,
            status,
            detail,
        });
    } else {
        has_fail = true;
        checks.push(EvidenceCheck {
            name: policy.advisory_receipt.clone(),
            path: advisory_file.display().to_string(),
            required: true,
            status: EvidenceStatus::Fail,
            detail: "missing advisory receipt".to_string(),
        });
    }

    let status = if has_fail {
        EvidenceStatus::Fail
    } else if has_warning {
        EvidenceStatus::Warning
    } else {
        EvidenceStatus::Pass
    };

    Ok(ReleaseEvidenceSummary {
        version: version.to_string(),
        bundle_path: out.display().to_string(),
        generated_at_utc: chrono::Utc::now().to_rfc3339(),
        status,
        checks,
    })
}

fn map_status(status: &str) -> EvidenceStatus {
    match status.to_ascii_lowercase().as_str() {
        "pass" | "ok" | "success" => EvidenceStatus::Pass,
        "warning" | "warn" => EvidenceStatus::Warning,
        _ => EvidenceStatus::Fail,
    }
}

fn read_receipt_status(path: &Path) -> Result<String> {
    let raw = fs::read_to_string(path)
        .with_context(|| format!("failed to read receipt {}", path.display()))?;
    let receipt: GenericReceipt = serde_json::from_str(&raw)
        .with_context(|| format!("failed to parse JSON receipt {}", path.display()))?;
    Ok(receipt.status.unwrap_or_else(|| "fail".to_string()))
}

fn write_summary(path: &Path, summary: &ReleaseEvidenceSummary) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("failed to create summary dir {}", parent.display()))?;
    }
    let json = serde_json::to_string_pretty(summary).context("failed to serialize summary")?;
    fs::write(path, json)
        .with_context(|| format!("failed to write summary receipt {}", path.display()))
}

struct ResolvedPolicy {
    required_receipts: Vec<String>,
    advisory_receipt: String,
    advisory_release_blocking: bool,
    summary_receipt: String,
}

fn load_policy(root: &Path) -> Result<ResolvedPolicy> {
    let policy_path = root.join(POLICY_PATH);
    let raw = fs::read_to_string(&policy_path)
        .with_context(|| format!("failed to read policy file {}", policy_path.display()))?;
    let file: EvidencePolicyFile =
        toml::from_str(&raw).context("failed to parse evidence policy")?;

    let release = file.release_evidence.context("missing [release_evidence] table in policy")?;

    let advisory = file.advisory.unwrap_or(AdvisoryPolicy {
        receipt: Some("advisory-status.json".to_string()),
        release_blocking: Some(false),
    });

    let mut dedup = BTreeMap::<String, ()>::new();
    for name in release.required {
        dedup.insert(name, ());
    }

    Ok(ResolvedPolicy {
        required_receipts: dedup.into_keys().collect(),
        advisory_receipt: advisory.receipt.unwrap_or_else(|| "advisory-status.json".to_string()),
        advisory_release_blocking: advisory.release_blocking.unwrap_or(false),
        summary_receipt: release
            .summary_receipt
            .unwrap_or_else(|| DEFAULT_SUMMARY_RECEIPT.to_string()),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use color_eyre::eyre::Result;
    use std::collections::BTreeMap;

    #[derive(Debug, Deserialize)]
    struct Fixture {
        receipts: BTreeMap<String, serde_json::Value>,
    }

    fn load_fixture(path: &Path) -> Result<Fixture> {
        let raw = fs::read_to_string(path)?;
        Ok(serde_json::from_str(&raw)?)
    }

    fn materialize_bundle(bundle_dir: &Path, fixture: &Fixture) -> Result<()> {
        fs::create_dir_all(bundle_dir)?;
        for (name, json) in &fixture.receipts {
            let path = bundle_dir.join(name);
            if let Some(parent) = path.parent() {
                fs::create_dir_all(parent)?;
            }
            fs::write(path, serde_json::to_string_pretty(json)?)?;
        }
        Ok(())
    }

    fn test_policy() -> ResolvedPolicy {
        ResolvedPolicy {
            required_receipts: vec![
                "ci-gate.json".to_string(),
                "parser-ratchet-release.json".to_string(),
                "vscode-extension-smoke.json".to_string(),
                "lsp-scenario.json".to_string(),
                "real-workspace-baseline.json".to_string(),
                "ai-completion-e2e.json".to_string(),
                "unresolved-risk-register.json".to_string(),
            ],
            advisory_receipt: "advisory-status.json".to_string(),
            advisory_release_blocking: false,
            summary_receipt: DEFAULT_SUMMARY_RECEIPT.to_string(),
        }
    }

    #[test]
    fn fixture_complete_bundle_passes() -> Result<()> {
        let tmp = tempfile::tempdir()?;
        let bundle = tmp.path().join("bundle");
        let fixture =
            load_fixture(Path::new("tests/fixtures/release-evidence/complete-bundle.json"))?;
        materialize_bundle(&bundle, &fixture)?;

        let summary = evaluate_bundle("0.13.0", &bundle, &test_policy())?;
        assert_eq!(summary.status, EvidenceStatus::Pass);
        Ok(())
    }

    #[test]
    fn fixture_missing_parser_ratchet_release_fails() -> Result<()> {
        let tmp = tempfile::tempdir()?;
        let bundle = tmp.path().join("bundle");
        let fixture = load_fixture(Path::new(
            "tests/fixtures/release-evidence/missing-parser-ratchet-release.json",
        ))?;
        materialize_bundle(&bundle, &fixture)?;

        let summary = evaluate_bundle("0.13.0", &bundle, &test_policy())?;
        assert_eq!(summary.status, EvidenceStatus::Fail);
        Ok(())
    }

    #[test]
    fn fixture_advisory_failure_produces_classified_warning() -> Result<()> {
        let tmp = tempfile::tempdir()?;
        let bundle = tmp.path().join("bundle");
        let fixture = load_fixture(Path::new(
            "tests/fixtures/release-evidence/advisory-failure-warning.json",
        ))?;
        materialize_bundle(&bundle, &fixture)?;

        let summary = evaluate_bundle("0.13.0", &bundle, &test_policy())?;
        assert_eq!(summary.status, EvidenceStatus::Warning);
        let advisory = summary
            .checks
            .iter()
            .find(|check| check.name == "advisory-status.json")
            .context("missing advisory check")?;
        assert_eq!(advisory.status, EvidenceStatus::Warning);
        Ok(())
    }
}
