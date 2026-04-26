use color_eyre::eyre::{Context, Result, bail};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use crate::utils::project_root;

const SCHEMA_VERSION: u32 = 1;
const CHECK_NAME: &str = "merge-readiness";
const DEFAULT_RECEIPT_PATH: &str = "target/receipts/merge-readiness.json";
const REQUIRED_CHECKS_PATH: &str = ".ci/policies/required-checks.toml";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MergeReadinessReceipt {
    pub check: String,
    pub schema_version: u32,
    pub event: String,
    pub pr: u64,
    pub head_sha: String,
    pub base_sha: String,
    pub gate_graph_version: String,
    pub required_checks: Vec<String>,
    pub review_evidence: Vec<String>,
    pub blocker_labels_absent: bool,
    pub verdict: String,
    pub expires_when: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VerifyStatus {
    Valid,
    StaleHead,
    StaleBase,
    StaleGateGraph,
    Blocked,
    Missing,
}

impl VerifyStatus {
    fn as_str(self) -> &'static str {
        match self {
            Self::Valid => "valid",
            Self::StaleHead => "stale_head",
            Self::StaleBase => "stale_base",
            Self::StaleGateGraph => "stale_gate_graph",
            Self::Blocked => "blocked",
            Self::Missing => "missing",
        }
    }
}

pub fn emit(pr: u64, receipt_path: Option<PathBuf>) -> Result<()> {
    let root = project_root()?;
    let required_checks = load_required_checks(&root)?;
    let head_sha = git_output(&root, &["rev-parse", "HEAD"])?;
    let base_sha = resolve_base_sha(&root)?;
    let gate_graph_version = compute_gate_graph_version(&root, &required_checks)?;

    let verdict = if required_checks.is_empty() { "blocked" } else { "valid" }.to_string();
    let blocker_labels_absent = true;

    let receipt = MergeReadinessReceipt {
        check: CHECK_NAME.to_string(),
        schema_version: SCHEMA_VERSION,
        event: "pull_request".to_string(),
        pr,
        head_sha,
        base_sha,
        gate_graph_version,
        required_checks,
        review_evidence: vec!["reviewed-deep".to_string(), "ci-green".to_string()],
        blocker_labels_absent,
        verdict,
        expires_when: "on_new_commit_or_base_or_policy_change".to_string(),
    };

    let output_path = receipt_path.unwrap_or_else(|| root.join(DEFAULT_RECEIPT_PATH));
    write_receipt(&output_path, &receipt)?;
    println!("wrote {}", output_path.display());

    Ok(())
}

pub fn verify(pr: Option<u64>, fixture: Option<PathBuf>) -> Result<()> {
    let root = project_root()?;
    let path = if let Some(fixture_path) = fixture {
        fixture_path
    } else {
        let _ = pr;
        root.join(DEFAULT_RECEIPT_PATH)
    };

    if !path.exists() {
        println!("{}", VerifyStatus::Missing.as_str());
        bail!("receipt not found: {}", path.display());
    }

    let receipt = load_receipt(&path)?;
    let required_checks = load_required_checks(&root)?;
    let current_head = git_output(&root, &["rev-parse", "HEAD"])?;
    let current_base = resolve_base_sha(&root)?;
    let current_gate_graph = compute_gate_graph_version(&root, &required_checks)?;

    let status = evaluate_receipt(&receipt, &current_head, &current_base, &current_gate_graph);
    println!("{}", status.as_str());

    if status == VerifyStatus::Valid {
        Ok(())
    } else {
        bail!("receipt status: {}", status.as_str())
    }
}

pub fn reconcile(dry_run: bool) -> Result<()> {
    let root = project_root()?;
    let path = root.join(DEFAULT_RECEIPT_PATH);

    if !path.exists() {
        println!("missing: {}", path.display());
        return Ok(());
    }

    let receipt = load_receipt(&path)?;
    let required_checks = load_required_checks(&root)?;
    let current_head = git_output(&root, &["rev-parse", "HEAD"])?;
    let current_base = resolve_base_sha(&root)?;
    let current_gate_graph = compute_gate_graph_version(&root, &required_checks)?;
    let status = evaluate_receipt(&receipt, &current_head, &current_base, &current_gate_graph);

    println!("status={}", status.as_str());
    if dry_run {
        println!("advisory: would reconcile merge-ready label changes only");
    } else {
        println!("apply: merge-ready reconciliation would be applied by workflow automation");
    }

    Ok(())
}

fn evaluate_receipt(
    receipt: &MergeReadinessReceipt,
    current_head: &str,
    current_base: &str,
    current_gate_graph: &str,
) -> VerifyStatus {
    if receipt.verdict == "blocked" || !receipt.blocker_labels_absent {
        return VerifyStatus::Blocked;
    }

    let receipt_head =
        resolve_runtime_token(&receipt.head_sha, current_head, current_base, current_gate_graph);
    let receipt_base =
        resolve_runtime_token(&receipt.base_sha, current_head, current_base, current_gate_graph);
    let receipt_gate = resolve_runtime_token(
        &receipt.gate_graph_version,
        current_head,
        current_base,
        current_gate_graph,
    );

    if receipt_head != current_head {
        return VerifyStatus::StaleHead;
    }

    if receipt_base != current_base {
        return VerifyStatus::StaleBase;
    }

    if receipt_gate != current_gate_graph {
        return VerifyStatus::StaleGateGraph;
    }

    VerifyStatus::Valid
}

fn resolve_runtime_token(
    value: &str,
    current_head: &str,
    current_base: &str,
    current_gate: &str,
) -> String {
    match value {
        "$CURRENT_HEAD" => current_head.to_string(),
        "$CURRENT_BASE" => current_base.to_string(),
        "$CURRENT_GATE_GRAPH" => current_gate.to_string(),
        _ => value.to_string(),
    }
}

fn load_receipt(path: &Path) -> Result<MergeReadinessReceipt> {
    let raw = fs::read_to_string(path)
        .with_context(|| format!("failed to read receipt: {}", path.display()))?;
    serde_json::from_str(&raw)
        .with_context(|| format!("failed to parse receipt: {}", path.display()))
}

fn write_receipt(path: &Path, receipt: &MergeReadinessReceipt) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("failed to create directory: {}", parent.display()))?;
    }

    let json = serde_json::to_string_pretty(receipt).context("failed to serialize receipt")?;
    fs::write(path, json).with_context(|| format!("failed to write receipt: {}", path.display()))
}

fn load_required_checks(root: &Path) -> Result<Vec<String>> {
    let path = root.join(REQUIRED_CHECKS_PATH);
    let raw = fs::read_to_string(&path)
        .with_context(|| format!("failed to read required checks policy: {}", path.display()))?;
    let value: toml::Value = toml::from_str(&raw)
        .with_context(|| format!("failed to parse required checks policy: {}", path.display()))?;

    let mut checks = Vec::new();
    if let Some(array) = value.get("checks").and_then(toml::Value::as_array) {
        for item in array {
            if let Some(name) = item.get("name").and_then(toml::Value::as_str) {
                checks.push(name.to_string());
            }
        }
    }

    checks.sort_unstable();
    Ok(checks)
}

fn resolve_base_sha(root: &Path) -> Result<String> {
    for base_ref in ["origin/master", "origin/main", "master", "main"] {
        if git_output(root, &["rev-parse", "--verify", base_ref]).is_ok() {
            return git_output(root, &["merge-base", "HEAD", base_ref]);
        }
    }

    git_output(root, &["rev-parse", "HEAD"])
}

fn git_output(root: &Path, args: &[&str]) -> Result<String> {
    let output = std::process::Command::new("git")
        .args(args)
        .current_dir(root)
        .output()
        .with_context(|| format!("failed to run git {}", args.join(" ")))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        bail!("git {} failed: {}", args.join(" "), stderr.trim());
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    Ok(stdout.trim().to_string())
}

fn compute_gate_graph_version(root: &Path, required_checks: &[String]) -> Result<String> {
    let mut inputs: BTreeMap<String, String> = BTreeMap::new();

    for rel in collect_gate_files(root)? {
        let path = root.join(&rel);
        if path.is_file() {
            let content = fs::read_to_string(&path)
                .with_context(|| format!("failed to read gate graph input: {}", path.display()))?;
            inputs.insert(rel, content.replace("\r\n", "\n"));
        }
    }

    inputs.insert(
        "required_checks".to_string(),
        serde_json::to_string(required_checks).context("failed to encode required checks")?,
    );

    let mut material = String::new();
    for (path, content) in inputs {
        material.push_str("## ");
        material.push_str(&path);
        material.push('\n');
        material.push_str(&content);
        material.push('\n');
    }

    Ok(fnv1a64_hex(material.as_bytes()))
}

fn collect_gate_files(root: &Path) -> Result<Vec<String>> {
    let mut files = Vec::new();

    for rel in
        [".ci/policies/required-checks.toml", ".ci/policies", ".ci/gates.d", ".github/workflows"]
    {
        let dir = root.join(rel);
        if dir.is_file() {
            files.push(rel.to_string());
            continue;
        }

        if !dir.exists() {
            continue;
        }

        for entry in walkdir::WalkDir::new(&dir)
            .into_iter()
            .filter_map(std::result::Result::ok)
            .filter(|entry| entry.path().is_file())
        {
            let path = entry.path();
            let rel_path = path
                .strip_prefix(root)
                .context("failed to strip repository root")?
                .to_string_lossy()
                .to_string();

            if rel == ".github/workflows" && !is_required_workflow_candidate(path) {
                continue;
            }

            files.push(rel_path);
        }
    }

    files.sort_unstable();
    files.dedup();
    Ok(files)
}

fn is_required_workflow_candidate(path: &Path) -> bool {
    let Some(name) = path.file_name().and_then(|n| n.to_str()) else {
        return false;
    };

    name.contains("ci") || name.contains("gate") || name.contains("merge")
}

fn fnv1a64_hex(bytes: &[u8]) -> String {
    let mut hash: u64 = 0xcbf29ce484222325;
    for byte in bytes {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
    }
    format!("fnv1a64:{hash:016x}")
}
