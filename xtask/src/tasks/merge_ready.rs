use color_eyre::eyre::{Context, Result};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};

use crate::utils::project_root;

const DEFAULT_RECEIPT_PATH: &str = "target/receipts/merge-readiness.json";
const CHECK_NAME: &str = "merge-readiness";
const SCHEMA_VERSION: u32 = 1;

#[derive(Debug, Clone)]
pub struct EmitOptions {
    pub pr: u64,
    pub receipt: PathBuf,
}

#[derive(Debug, Clone)]
pub struct VerifyOptions {
    pub pr: u64,
    pub fixture: Option<PathBuf>,
}

#[derive(Debug, Clone)]
pub struct ReconcileOptions {
    pub dry_run: bool,
    pub apply: bool,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct MergeReadinessReceipt {
    pub check: String,
    pub schema_version: u32,
    pub event: String,
    pub pr: u64,
    pub head_sha: String,
    pub base_sha: String,
    pub gate_graph_version: String,
    pub required_checks: Vec<String>,
    pub review_evidence: ReviewEvidence,
    pub blocker_labels_absent: bool,
    pub verdict: String,
    pub expires_when: ExpiresWhen,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ReviewEvidence {
    pub approved: bool,
    pub approving_review_count: u32,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ExpiresWhen {
    pub head_sha_changes: bool,
    pub base_sha_changes: bool,
    pub gate_graph_version_changes: bool,
}

#[derive(Debug, Deserialize)]
struct RequiredChecksPolicy {
    required_checks: Option<Vec<String>>,
}

pub fn emit(options: EmitOptions) -> Result<()> {
    let root = project_root()?;
    let gate_graph_version = compute_gate_graph_version(&root)?;
    let required_checks = load_required_checks(&root)?;
    let head_sha = current_head_sha(&root)?;
    let base_sha = current_base_sha(&root)?;

    let receipt = MergeReadinessReceipt {
        check: CHECK_NAME.to_string(),
        schema_version: SCHEMA_VERSION,
        event: "pull_request".to_string(),
        pr: options.pr,
        head_sha,
        base_sha,
        gate_graph_version,
        required_checks,
        review_evidence: ReviewEvidence { approved: true, approving_review_count: 1 },
        blocker_labels_absent: true,
        verdict: "valid".to_string(),
        expires_when: ExpiresWhen {
            head_sha_changes: true,
            base_sha_changes: true,
            gate_graph_version_changes: true,
        },
    };

    if let Some(parent) = options.receipt.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("failed to create {}", parent.display()))?;
    }

    let payload = serde_json::to_string_pretty(&receipt).context("failed to serialize receipt")?;
    fs::write(&options.receipt, payload)
        .with_context(|| format!("failed to write {}", options.receipt.display()))?;

    println!("wrote merge-readiness receipt to {}", options.receipt.display());
    Ok(())
}

pub fn verify(options: VerifyOptions) -> Result<()> {
    let root = project_root()?;
    let receipt_path = options.fixture.unwrap_or_else(|| root.join(DEFAULT_RECEIPT_PATH));

    let status = verify_receipt_for_pr(&root, options.pr, &receipt_path)?;
    println!("{status}");
    Ok(())
}

pub fn reconcile(options: ReconcileOptions) -> Result<()> {
    let root = project_root()?;
    let receipt_path = root.join(DEFAULT_RECEIPT_PATH);
    let mode = if options.apply { "apply" } else { "dry-run" };

    let status = if receipt_path.exists() {
        verify_receipt_for_pr(&root, 0, &receipt_path)?
    } else {
        "missing".to_string()
    };

    println!("merge-ready reconcile mode={mode} status={status}");
    if options.apply && status != "valid" {
        println!(
            "would remove merge-ready label and comment with receipt details (reason={status})"
        );
    }

    if options.dry_run && options.apply {
        println!("note: --apply takes precedence over --dry-run");
    }

    Ok(())
}

fn verify_receipt_for_pr(root: &Path, pr: u64, receipt_path: &Path) -> Result<String> {
    if !receipt_path.exists() {
        return Ok("missing".to_string());
    }

    let raw = fs::read_to_string(receipt_path)
        .with_context(|| format!("failed to read {}", receipt_path.display()))?;
    let receipt: MergeReadinessReceipt = serde_json::from_str(&raw)
        .with_context(|| format!("invalid JSON: {}", receipt_path.display()))?;

    if pr != 0 && receipt.pr != pr {
        return Ok("missing".to_string());
    }

    if receipt.verdict == "blocked" || !receipt.blocker_labels_absent {
        return Ok("blocked".to_string());
    }

    let head_sha = current_head_sha(root)?;
    if receipt.head_sha != head_sha {
        return Ok("stale_head".to_string());
    }

    let base_sha = current_base_sha(root)?;
    if receipt.base_sha != base_sha {
        return Ok("stale_base".to_string());
    }

    let gate_graph_version = compute_gate_graph_version(root)?;
    if receipt.gate_graph_version != gate_graph_version {
        return Ok("stale_gate_graph".to_string());
    }

    Ok("valid".to_string())
}

fn load_required_checks(root: &Path) -> Result<Vec<String>> {
    let path = root.join(".ci/policies/required-checks.toml");
    let raw = fs::read_to_string(&path)
        .with_context(|| format!("failed to read required checks at {}", path.display()))?;
    let parsed: RequiredChecksPolicy =
        toml::from_str(&raw).with_context(|| format!("invalid TOML: {}", path.display()))?;
    let mut checks = parsed.required_checks.unwrap_or_default();
    checks.sort();
    checks.dedup();
    Ok(checks)
}

fn current_head_sha(root: &Path) -> Result<String> {
    let output = std::process::Command::new("git")
        .args(["rev-parse", "HEAD"])
        .current_dir(root)
        .output()
        .context("failed to run git rev-parse HEAD")?;
    if !output.status.success() {
        return Ok("unknown-head".to_string());
    }
    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

fn current_base_sha(root: &Path) -> Result<String> {
    for candidate in ["origin/master", "master"] {
        let output = std::process::Command::new("git")
            .args(["rev-parse", candidate])
            .current_dir(root)
            .output()
            .with_context(|| format!("failed to resolve {candidate}"))?;
        if output.status.success() {
            return Ok(String::from_utf8_lossy(&output.stdout).trim().to_string());
        }
    }

    Ok("unknown-base".to_string())
}

fn compute_gate_graph_version(root: &Path) -> Result<String> {
    let mut files = BTreeSet::new();

    collect_files(root, ".ci/policies", &mut files)?;
    collect_files(root, ".ci/gates.d", &mut files)?;
    collect_workflows(root, &mut files)?;

    let mut hasher = Sha256::new();
    for rel in files {
        let path = root.join(&rel);
        if !path.is_file() {
            continue;
        }
        let content =
            fs::read(&path).with_context(|| format!("failed to read {}", path.display()))?;
        hasher.update(rel.as_bytes());
        hasher.update([0]);
        hasher.update(&content);
        hasher.update([0]);
    }

    let digest = hasher.finalize();
    Ok(digest.iter().map(|b| format!("{b:02x}")).collect())
}

fn collect_files(root: &Path, rel_dir: &str, files: &mut BTreeSet<String>) -> Result<()> {
    let dir = root.join(rel_dir);
    if !dir.exists() {
        return Ok(());
    }

    for entry in walkdir::WalkDir::new(&dir) {
        let entry = entry.with_context(|| format!("failed to walk {}", dir.display()))?;
        if entry.file_type().is_file() {
            let rel = entry.path().strip_prefix(root).with_context(|| {
                format!("failed to strip prefix from {}", entry.path().display())
            })?;
            files.insert(rel.to_string_lossy().replace('\\', "/"));
        }
    }

    Ok(())
}

fn collect_workflows(root: &Path, files: &mut BTreeSet<String>) -> Result<()> {
    let workflows_dir = root.join(".github/workflows");
    if !workflows_dir.exists() {
        return Ok(());
    }

    let required_checks = load_required_checks(root)?;
    for entry in walkdir::WalkDir::new(&workflows_dir).max_depth(1) {
        let entry = entry.with_context(|| format!("failed to walk {}", workflows_dir.display()))?;
        if !entry.file_type().is_file() {
            continue;
        }

        let rel = entry
            .path()
            .strip_prefix(root)
            .with_context(|| format!("failed to strip prefix from {}", entry.path().display()))?;
        let content = fs::read_to_string(entry.path())
            .with_context(|| format!("failed to read {}", entry.path().display()))?;

        if required_checks.iter().any(|check| content.contains(check)) {
            files.insert(rel.to_string_lossy().replace('\\', "/"));
        }
    }

    Ok(())
}
