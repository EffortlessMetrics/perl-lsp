use color_eyre::eyre::{Context, Result, bail};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use crate::utils::project_root;

const DEFAULT_POLICY_PATH: &str = ".ci/release/evidence.toml";
const STATUS_PASS: &str = "pass";

#[derive(Debug, Deserialize)]
struct EvidencePolicy {
    schema_version: u32,
    summary_receipt: String,
    required: Vec<RequiredEvidence>,
}

#[derive(Debug, Deserialize)]
struct RequiredEvidence {
    name: String,
    path: String,
    #[serde(default)]
    kind: EvidenceKind,
    #[serde(default)]
    release_blocking: bool,
}

#[derive(Debug, Deserialize, Clone, Copy, PartialEq, Eq, Default)]
#[serde(rename_all = "kebab-case")]
enum EvidenceKind {
    #[default]
    Required,
    Advisory,
}

#[derive(Debug, Deserialize)]
struct SourceReceipt {
    status: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ReleaseEvidenceReceipt {
    schema_version: u32,
    version: String,
    out_dir: String,
    overall_status: String,
    items: Vec<ReleaseEvidenceItem>,
    warnings: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize)]
struct ReleaseEvidenceItem {
    name: String,
    path: String,
    kind: String,
    status: String,
    release_blocking: bool,
    classification: String,
}

pub fn generate(version: String, out: PathBuf) -> Result<()> {
    let root = project_root()?;
    let policy = load_policy(&root)?;

    let report = evaluate_bundle(&policy, &version, &out)?;

    let summary_path = root.join(policy.summary_receipt);
    if let Some(parent) = summary_path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("Failed to create {}", parent.display()))?;
    }
    let json = serde_json::to_string_pretty(&report).context("Failed to serialize summary")?;
    fs::write(&summary_path, json)
        .with_context(|| format!("Failed to write summary to {}", summary_path.display()))?;

    if report.overall_status != STATUS_PASS {
        bail!("Release evidence incomplete. Summary receipt: {}", summary_path.display());
    }

    println!("Release evidence summary written: {}", summary_path.display());
    Ok(())
}

pub fn verify(version: String, receipt: PathBuf) -> Result<()> {
    let root = project_root()?;
    let policy = load_policy(&root)?;

    let receipt_path = if receipt.is_absolute() { receipt } else { root.join(receipt) };
    let content = fs::read_to_string(&receipt_path)
        .with_context(|| format!("Failed to read {}", receipt_path.display()))?;
    let parsed: ReleaseEvidenceReceipt =
        serde_json::from_str(&content).context("Failed to parse release evidence receipt JSON")?;

    if parsed.version != version {
        bail!("Receipt version mismatch: expected {version}, found {}", parsed.version);
    }

    let mut by_name: BTreeMap<&str, &ReleaseEvidenceItem> = BTreeMap::new();
    for item in &parsed.items {
        by_name.insert(item.name.as_str(), item);
    }

    let mut errors: Vec<String> = Vec::new();
    let mut advisory_warnings: Vec<String> = Vec::new();

    for required in &policy.required {
        let Some(item) = by_name.get(required.name.as_str()) else {
            errors.push(format!("Missing required receipt entry: {}", required.name));
            continue;
        };

        if item.status != STATUS_PASS {
            if required.kind == EvidenceKind::Advisory && !required.release_blocking {
                advisory_warnings
                    .push(format!("Advisory '{}' is failing but non-blocking", required.name));
            } else {
                errors.push(format!(
                    "Receipt '{}' is not pass (status={})",
                    required.name, item.status
                ));
            }
        }
    }

    if !advisory_warnings.is_empty() {
        for warning in advisory_warnings {
            println!("warning: {warning}");
        }
    }

    if !errors.is_empty() {
        bail!(errors.join("; "));
    }

    println!("Release evidence receipt verified: {}", receipt_path.display());
    Ok(())
}

fn load_policy(root: &Path) -> Result<EvidencePolicy> {
    let policy_path = root.join(DEFAULT_POLICY_PATH);
    let content = fs::read_to_string(&policy_path)
        .with_context(|| format!("Failed to read {}", policy_path.display()))?;
    let parsed: EvidencePolicy = toml::from_str(&content)
        .with_context(|| format!("Failed to parse {}", policy_path.display()))?;
    if parsed.schema_version != 1 {
        bail!("Unsupported release evidence policy schema: {}", parsed.schema_version);
    }
    Ok(parsed)
}

fn evaluate_bundle(
    policy: &EvidencePolicy,
    version: &str,
    out_dir: &Path,
) -> Result<ReleaseEvidenceReceipt> {
    let mut items: Vec<ReleaseEvidenceItem> = Vec::new();
    let mut warnings: Vec<String> = Vec::new();
    let mut blocking_failures = 0usize;

    for required in &policy.required {
        let receipt_path = out_dir.join(&required.path);
        if !receipt_path.exists() {
            blocking_failures += 1;
            items.push(ReleaseEvidenceItem {
                name: required.name.clone(),
                path: required.path.clone(),
                kind: kind_name(required.kind).to_string(),
                status: "missing".to_string(),
                release_blocking: required.release_blocking
                    || required.kind == EvidenceKind::Required,
                classification: "missing".to_string(),
            });
            continue;
        }

        let content = fs::read_to_string(&receipt_path)
            .with_context(|| format!("Failed to read {}", receipt_path.display()))?;
        let source: SourceReceipt = serde_json::from_str(&content).with_context(|| {
            format!("Receipt {} must include a string `status` field", receipt_path.display())
        })?;

        let classification = classify(required, &source.status);

        if source.status != STATUS_PASS {
            match classification.as_str() {
                "advisory-warning" => warnings
                    .push(format!("{} failing is classified as advisory warning", required.name)),
                _ => blocking_failures += 1,
            }
        }

        items.push(ReleaseEvidenceItem {
            name: required.name.clone(),
            path: required.path.clone(),
            kind: kind_name(required.kind).to_string(),
            status: source.status,
            release_blocking: required.release_blocking || required.kind == EvidenceKind::Required,
            classification,
        });
    }

    let overall_status = if blocking_failures == 0 { STATUS_PASS } else { "fail" }.to_string();

    Ok(ReleaseEvidenceReceipt {
        schema_version: 1,
        version: version.to_string(),
        out_dir: out_dir.display().to_string(),
        overall_status,
        items,
        warnings,
    })
}

fn classify(required: &RequiredEvidence, status: &str) -> String {
    if status == STATUS_PASS {
        return "pass".to_string();
    }

    if required.kind == EvidenceKind::Advisory && !required.release_blocking {
        return "advisory-warning".to_string();
    }

    "release-blocking".to_string()
}

fn kind_name(kind: EvidenceKind) -> &'static str {
    match kind {
        EvidenceKind::Required => "required",
        EvidenceKind::Advisory => "advisory",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use color_eyre::eyre::eyre;

    fn fixtures_dir() -> Result<PathBuf> {
        let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let dir = manifest.join("tests/fixtures/release-evidence");
        if dir.exists() {
            Ok(dir)
        } else {
            Err(eyre!("Fixtures directory missing: {}", dir.display()))
        }
    }

    #[test]
    fn fixture_complete_bundle_passes() -> Result<()> {
        let fixture = fixtures_dir()?.join("complete-bundle.json");
        let payload = fs::read_to_string(&fixture)?;
        let parsed: ReleaseEvidenceReceipt = serde_json::from_str(&payload)?;
        assert_eq!(parsed.overall_status, STATUS_PASS);
        Ok(())
    }

    #[test]
    fn fixture_missing_parser_ratchet_release_fails() -> Result<()> {
        let fixture = fixtures_dir()?.join("missing-parser-ratchet-release.json");
        let payload = fs::read_to_string(&fixture)?;
        let parsed: ReleaseEvidenceReceipt = serde_json::from_str(&payload)?;

        assert_eq!(parsed.overall_status, "fail");

        let found = parsed
            .items
            .iter()
            .any(|item| item.name == "parser-ratchet-release" && item.status == "missing");
        assert!(found, "fixture must mark parser-ratchet-release as missing");
        Ok(())
    }

    #[test]
    fn fixture_advisory_failure_is_classified_warning() -> Result<()> {
        let fixture = fixtures_dir()?.join("advisory-failure-warning.json");
        let payload = fs::read_to_string(&fixture)?;
        let parsed: ReleaseEvidenceReceipt = serde_json::from_str(&payload)?;

        let item = parsed
            .items
            .iter()
            .find(|item| item.name == "advisory-status")
            .ok_or_else(|| eyre!("advisory-status fixture entry missing"))?;

        assert_eq!(item.classification, "advisory-warning");
        Ok(())
    }
}
