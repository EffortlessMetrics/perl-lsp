use color_eyre::eyre::{Context, Result, bail};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::fs;
use std::path::Path;

use crate::utils::project_root;

const DEFAULT_POLICY_PATH: &str = ".ci/release/evidence.toml";

#[derive(Debug, Deserialize)]
struct EvidencePolicy {
    receipts: Vec<ReceiptPolicy>,
}

#[derive(Debug, Deserialize, Clone)]
struct ReceiptPolicy {
    name: String,
    file: String,
    #[serde(default = "default_true")]
    required: bool,
    #[serde(default = "default_true")]
    release_blocking: bool,
}

#[derive(Debug, Serialize)]
struct EvidenceScaffold {
    version: String,
    bundle_dir: String,
    required_receipts: Vec<ScaffoldReceipt>,
}

#[derive(Debug, Serialize)]
struct ScaffoldReceipt {
    name: String,
    path: String,
    required: bool,
    release_blocking: bool,
}

#[derive(Debug, Serialize)]
pub struct VerifySummary {
    version: String,
    bundle_dir: String,
    status: &'static str,
    blocking_failures: Vec<String>,
    warnings: Vec<String>,
    receipts: Vec<ReceiptResult>,
}

#[derive(Debug, Serialize)]
struct ReceiptResult {
    name: String,
    path: String,
    status: String,
    required: bool,
    release_blocking: bool,
    classification: String,
}

fn default_true() -> bool {
    true
}

pub fn scaffold(version: &str, out_dir: &Path) -> Result<()> {
    let root = project_root()?;
    let policy = load_policy(&root)?;
    fs::create_dir_all(out_dir)
        .with_context(|| format!("failed to create output dir {}", out_dir.display()))?;

    let bundle = EvidenceScaffold {
        version: version.to_string(),
        bundle_dir: out_dir.display().to_string(),
        required_receipts: policy
            .receipts
            .iter()
            .map(|receipt| ScaffoldReceipt {
                name: receipt.name.clone(),
                path: out_dir.join(&receipt.file).display().to_string(),
                required: receipt.required,
                release_blocking: receipt.release_blocking,
            })
            .collect(),
    };

    let manifest_path = out_dir.join("required-receipts.json");
    let rendered = serde_json::to_string_pretty(&bundle)?;
    fs::write(&manifest_path, rendered)
        .with_context(|| format!("failed writing {}", manifest_path.display()))?;
    println!("release evidence scaffold written: {}", manifest_path.display());
    Ok(())
}

pub fn verify(version: &str, bundle_dir: &Path, receipt_path: &Path) -> Result<()> {
    let root = project_root()?;
    let policy = load_policy(&root)?;
    let mut blocking_failures = Vec::new();
    let mut warnings = Vec::new();
    let mut receipts = Vec::new();

    for policy_receipt in policy.receipts {
        let receipt_file = bundle_dir.join(&policy_receipt.file);
        if !receipt_file.exists() {
            let message = format!("missing required receipt: {}", receipt_file.display());
            receipts.push(ReceiptResult {
                name: policy_receipt.name,
                path: receipt_file.display().to_string(),
                status: "missing".to_string(),
                required: policy_receipt.required,
                release_blocking: policy_receipt.release_blocking,
                classification: if policy_receipt.release_blocking {
                    "missing-blocking".to_string()
                } else {
                    "missing-warning".to_string()
                },
            });
            if policy_receipt.required && policy_receipt.release_blocking {
                blocking_failures.push(message);
            } else if policy_receipt.required {
                warnings.push(message);
            }
            continue;
        }

        let value: Value = serde_json::from_str(
            &fs::read_to_string(&receipt_file)
                .with_context(|| format!("failed reading {}", receipt_file.display()))?,
        )
        .with_context(|| format!("invalid json: {}", receipt_file.display()))?;

        let status = extract_status(&value).unwrap_or_else(|| "unknown".to_string());
        let is_pass = status.eq_ignore_ascii_case("pass");

        let classification = if is_pass {
            "pass".to_string()
        } else if policy_receipt.release_blocking {
            blocking_failures.push(format!("{} failed with status={status}", policy_receipt.name));
            "failure-blocking".to_string()
        } else {
            warnings.push(format!(
                "{} failed with status={status} (classified advisory warning)",
                policy_receipt.name
            ));
            "failure-advisory-warning".to_string()
        };

        receipts.push(ReceiptResult {
            name: policy_receipt.name,
            path: receipt_file.display().to_string(),
            status,
            required: policy_receipt.required,
            release_blocking: policy_receipt.release_blocking,
            classification,
        });
    }

    let overall_status = if blocking_failures.is_empty() { "pass" } else { "fail" };
    let summary = VerifySummary {
        version: version.to_string(),
        bundle_dir: bundle_dir.display().to_string(),
        status: overall_status,
        blocking_failures,
        warnings,
        receipts,
    };

    if let Some(parent) = receipt_path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("failed creating {}", parent.display()))?;
    }
    fs::write(receipt_path, serde_json::to_string_pretty(&summary)?)
        .with_context(|| format!("failed writing {}", receipt_path.display()))?;

    if summary.status == "fail" {
        bail!("release evidence verification failed; see {}", receipt_path.display());
    }

    println!("release evidence verification passed: {}", receipt_path.display());
    Ok(())
}

fn load_policy(root: &Path) -> Result<EvidencePolicy> {
    let path = root.join(DEFAULT_POLICY_PATH);
    let contents = fs::read_to_string(&path)
        .with_context(|| format!("failed reading policy {}", path.display()))?;
    toml::from_str(&contents).with_context(|| format!("invalid policy {}", path.display()))
}

fn extract_status(value: &Value) -> Option<String> {
    value
        .get("status")
        .and_then(Value::as_str)
        .map(ToString::to_string)
        .or_else(|| value.get("outcome").and_then(Value::as_str).map(ToString::to_string))
        .or_else(|| {
            value
                .get("success")
                .and_then(Value::as_bool)
                .map(|v| if v { "pass" } else { "fail" }.to_string())
        })
}
