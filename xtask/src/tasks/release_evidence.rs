use color_eyre::eyre::{Context, Result, bail};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};

use crate::utils::project_root;

const POLICY_PATH: &str = ".ci/release/evidence.toml";
const REQUIRED_RECEIPTS: [&str; 8] = [
    "ci-gate.json",
    "parser-ratchet-release.json",
    "vscode-extension-smoke.json",
    "lsp-scenario.json",
    "real-workspace-baseline.json",
    "ai-completion-e2e.json",
    "advisory-status.json",
    "unresolved-risk-register.json",
];

#[derive(Debug, Deserialize)]
struct EvidencePolicy {
    #[serde(default)]
    release_blocking_advisory_severities: Vec<String>,
}

#[derive(Debug, Serialize)]
struct ScaffoldReceipt<'a> {
    version: &'a str,
    out: String,
    required_receipts: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct GenericReceipt {
    status: String,
    #[serde(default)]
    advisories: Vec<AdvisoryFinding>,
}

#[derive(Debug, Deserialize)]
struct AdvisoryFinding {
    id: Option<String>,
    severity: Option<String>,
}

#[derive(Debug, Serialize)]
struct VerifyReceipt {
    subsystem: &'static str,
    version: String,
    bundle_root: String,
    required_receipts: Vec<String>,
    missing_receipts: Vec<String>,
    failed_receipts: Vec<String>,
    advisory_warnings: Vec<String>,
    status: String,
}

pub fn scaffold(version: String, out: PathBuf) -> Result<()> {
    fs::create_dir_all(&out)
        .with_context(|| format!("creating evidence bundle directory {}", out.display()))?;

    let receipt = ScaffoldReceipt {
        version: &version,
        out: out.display().to_string(),
        required_receipts: REQUIRED_RECEIPTS.iter().map(|name| (*name).to_string()).collect(),
    };

    let manifest = out.join("bundle-scaffold.json");
    fs::write(&manifest, serde_json::to_string_pretty(&receipt)?).with_context(|| {
        format!("writing release evidence scaffold manifest to {}", manifest.display())
    })?;

    println!("Scaffolded release evidence bundle at {}", out.display());
    Ok(())
}

pub fn verify(version: String, receipt_out: PathBuf) -> Result<()> {
    let root = project_root()?;
    let bundle_root = root.join("target").join("release-evidence").join(format!("v{version}"));
    let policy = load_policy(&root)?;

    let mut missing = Vec::new();
    let mut failed = Vec::new();
    let mut advisory_warnings = Vec::new();

    for name in REQUIRED_RECEIPTS {
        let path = bundle_root.join(name);
        if !path.exists() {
            missing.push(name.to_string());
            continue;
        }

        let parsed = parse_receipt(&path)?;
        let passed = parsed.status.eq_ignore_ascii_case("pass");

        if name == "advisory-status.json" {
            if !passed {
                let blocking = classify_advisory_failure(&parsed.advisories, &policy);
                if blocking.is_empty() {
                    advisory_warnings.push(
                        "advisory-status.json failed but no release-blocking policy match"
                            .to_string(),
                    );
                } else {
                    failed.push(name.to_string());
                    advisory_warnings.extend(blocking);
                }
            }
            continue;
        }

        if !passed {
            failed.push(name.to_string());
        }
    }

    let status = if missing.is_empty() && failed.is_empty() { "pass" } else { "fail" };

    let summary = VerifyReceipt {
        subsystem: "release_evidence",
        version,
        bundle_root: bundle_root.display().to_string(),
        required_receipts: REQUIRED_RECEIPTS.iter().map(|value| (*value).to_string()).collect(),
        missing_receipts: missing.clone(),
        failed_receipts: failed.clone(),
        advisory_warnings,
        status: status.to_string(),
    };

    if let Some(parent) = receipt_out.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("creating receipt directory {}", parent.display()))?;
    }
    fs::write(&receipt_out, serde_json::to_string_pretty(&summary)?).with_context(|| {
        format!("writing release-evidence summary receipt {}", receipt_out.display())
    })?;

    if !missing.is_empty() {
        bail!("release evidence verification failed: missing receipts: {}", missing.join(", "));
    }
    if !failed.is_empty() {
        bail!("release evidence verification failed: failing receipts: {}", failed.join(", "));
    }

    println!("Release evidence verified: {}", receipt_out.display());
    Ok(())
}

fn load_policy(root: &Path) -> Result<EvidencePolicy> {
    let path = root.join(POLICY_PATH);
    let contents = fs::read_to_string(&path)
        .with_context(|| format!("reading release evidence policy {}", path.display()))?;
    let policy = toml::from_str::<EvidencePolicy>(&contents)
        .with_context(|| format!("parsing release evidence policy {}", path.display()))?;
    Ok(policy)
}

fn parse_receipt(path: &Path) -> Result<GenericReceipt> {
    let contents = fs::read_to_string(path)
        .with_context(|| format!("reading evidence receipt {}", path.display()))?;
    let parsed = serde_json::from_str::<GenericReceipt>(&contents)
        .with_context(|| format!("parsing evidence receipt {}", path.display()))?;
    Ok(parsed)
}

fn classify_advisory_failure(
    advisories: &[AdvisoryFinding],
    policy: &EvidencePolicy,
) -> Vec<String> {
    let blocking: BTreeSet<String> = policy
        .release_blocking_advisory_severities
        .iter()
        .map(|value| value.to_ascii_lowercase())
        .collect();

    advisories
        .iter()
        .filter_map(|advisory| {
            let severity = advisory
                .severity
                .as_ref()
                .map(|value| value.to_ascii_lowercase())
                .unwrap_or_else(|| "unknown".to_string());

            if blocking.contains(&severity) {
                let id = advisory.id.as_deref().unwrap_or("unknown-advisory");
                Some(format!("release-blocking advisory: id={id}, severity={severity}"))
            } else {
                None
            }
        })
        .collect()
}
