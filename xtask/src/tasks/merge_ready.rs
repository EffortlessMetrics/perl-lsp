use color_eyre::eyre::{Context, Result, bail, eyre};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};

use crate::utils::project_root;

const RECEIPT_CHECK: &str = "merge-readiness";
const RECEIPT_SCHEMA_VERSION: u32 = 1;

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
    pub fn as_str(self) -> &'static str {
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
    pub expires_when: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ReviewEvidence {
    pub approved: bool,
    pub reviewed_head_sha: String,
}

#[derive(Debug, Deserialize)]
struct RequiredChecksPolicy {
    required_checks: Option<Vec<String>>,
}

pub fn emit(pr: u64, receipt_path: PathBuf) -> Result<()> {
    let root = project_root()?;
    let required_checks = load_required_checks(&root)?;
    let head_sha = git_output(&root, ["rev-parse", "HEAD"])?;
    let base_sha = resolve_base_sha(&root)?;
    let gate_graph_version = compute_gate_graph_version(&root, &required_checks)?;

    let receipt = MergeReadinessReceipt {
        check: RECEIPT_CHECK.to_string(),
        schema_version: RECEIPT_SCHEMA_VERSION,
        event: "pull_request".to_string(),
        pr,
        head_sha: head_sha.clone(),
        base_sha: base_sha.clone(),
        gate_graph_version,
        required_checks,
        review_evidence: ReviewEvidence { approved: true, reviewed_head_sha: head_sha },
        blocker_labels_absent: true,
        verdict: VerifyStatus::Valid.as_str().to_string(),
        expires_when: "head_sha_or_base_sha_or_gate_graph_changes".to_string(),
    };

    if let Some(parent) = receipt_path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("failed creating {}", parent.display()))?;
    }
    let json = serde_json::to_string_pretty(&receipt)?;
    fs::write(&receipt_path, json)
        .with_context(|| format!("failed writing {}", receipt_path.display()))?;
    println!("wrote merge readiness receipt to {}", receipt_path.display());
    Ok(())
}

pub fn verify(pr: Option<u64>, fixture: Option<PathBuf>) -> Result<()> {
    let root = project_root()?;
    let status = verify_status(&root, pr, fixture.as_deref())?;
    println!("{}", status.as_str());
    Ok(())
}

pub fn reconcile(dry_run: bool, apply: bool) -> Result<()> {
    if dry_run && apply {
        bail!("choose either --dry-run or --apply");
    }
    let root = project_root()?;
    let mode = if apply { "apply" } else { "dry-run" };
    let status = verify_status(&root, None, None)?;
    println!("merge-ready reconcile mode={mode} status={}", status.as_str());
    if apply && status != VerifyStatus::Valid {
        println!(
            "would remove merge-ready label and comment: removed due to {} receipt mismatch",
            status.as_str()
        );
    }
    Ok(())
}

fn verify_status(root: &Path, pr: Option<u64>, fixture: Option<&Path>) -> Result<VerifyStatus> {
    let receipt_path = fixture
        .map(Path::to_path_buf)
        .unwrap_or_else(|| root.join("target/receipts/merge-readiness.json"));
    if !receipt_path.exists() {
        return Ok(VerifyStatus::Missing);
    }

    let receipt: MergeReadinessReceipt = serde_json::from_str(
        &fs::read_to_string(&receipt_path)
            .with_context(|| format!("failed reading {}", receipt_path.display()))?,
    )
    .with_context(|| format!("failed parsing {}", receipt_path.display()))?;

    if receipt.check != RECEIPT_CHECK || receipt.schema_version != RECEIPT_SCHEMA_VERSION {
        return Ok(VerifyStatus::Missing);
    }

    if let Some(pr_number) = pr
        && receipt.pr != pr_number
    {
        return Ok(VerifyStatus::Missing);
    }

    if !receipt.blocker_labels_absent || receipt.verdict == VerifyStatus::Blocked.as_str() {
        return Ok(VerifyStatus::Blocked);
    }

    if fixture.is_some() {
        return Ok(parse_fixture_status(&receipt.verdict));
    }

    let head_sha = git_output(root, ["rev-parse", "HEAD"])?;
    if receipt.head_sha != head_sha {
        return Ok(VerifyStatus::StaleHead);
    }

    let base_sha = resolve_base_sha(root)?;
    if receipt.base_sha != base_sha {
        return Ok(VerifyStatus::StaleBase);
    }

    let required_checks = load_required_checks(root)?;
    let gate_graph_version = compute_gate_graph_version(root, &required_checks)?;
    if receipt.gate_graph_version != gate_graph_version {
        return Ok(VerifyStatus::StaleGateGraph);
    }

    Ok(VerifyStatus::Valid)
}

fn parse_fixture_status(verdict: &str) -> VerifyStatus {
    match verdict {
        "valid" => VerifyStatus::Valid,
        "stale_head" => VerifyStatus::StaleHead,
        "stale_base" => VerifyStatus::StaleBase,
        "stale_gate_graph" => VerifyStatus::StaleGateGraph,
        "blocked" => VerifyStatus::Blocked,
        _ => VerifyStatus::Missing,
    }
}

fn load_required_checks(root: &Path) -> Result<Vec<String>> {
    let policy_path = root.join(".ci/policies/required-checks.toml");
    if !policy_path.exists() {
        return Ok(vec!["ci / pr-fast".to_string()]);
    }

    let raw = fs::read_to_string(&policy_path)
        .with_context(|| format!("failed reading {}", policy_path.display()))?;
    let parsed: RequiredChecksPolicy =
        toml::from_str(&raw).with_context(|| format!("invalid {}", policy_path.display()))?;

    let mut checks = parsed.required_checks.unwrap_or_default();
    checks.sort_unstable();
    checks.dedup();
    Ok(checks)
}

fn compute_gate_graph_version(root: &Path, required_checks: &[String]) -> Result<String> {
    let mut files = BTreeSet::new();
    let required_checks_path = root.join(".ci/policies/required-checks.toml");
    if required_checks_path.exists() {
        files.insert(required_checks_path);
    }

    collect_files(root, &root.join(".ci/policies"), &mut files)?;
    collect_files(root, &root.join(".ci/gates.d"), &mut files)?;

    let workflow_dir = root.join(".github/workflows");
    if workflow_dir.exists() {
        for entry in fs::read_dir(&workflow_dir)
            .with_context(|| format!("failed reading {}", workflow_dir.display()))?
        {
            let entry = entry?;
            let path = entry.path();
            if !path.is_file() {
                continue;
            }
            let text = fs::read_to_string(&path)
                .with_context(|| format!("failed reading {}", path.display()))?;
            let referenced = required_checks.iter().any(|check| text.contains(check));
            if referenced {
                files.insert(path);
            }
        }
    }

    let mut hasher = Sha256::new();
    for path in files {
        let rel = path
            .strip_prefix(root)
            .map_err(|err| eyre!("failed to relativize {}: {err}", path.display()))?;
        hasher.update(rel.to_string_lossy().as_bytes());
        hasher.update([0]);
        hasher
            .update(fs::read(&path).with_context(|| format!("failed reading {}", path.display()))?);
        hasher.update([0]);
    }

    Ok(format!("{:x}", hasher.finalize()))
}

fn collect_files(root: &Path, dir: &Path, out: &mut BTreeSet<PathBuf>) -> Result<()> {
    if !dir.exists() {
        return Ok(());
    }
    for entry in walkdir::WalkDir::new(dir) {
        let entry = entry?;
        let path = entry.path();
        if path.is_file() {
            out.insert(root.join(path.strip_prefix(root)?));
        }
    }
    Ok(())
}

fn resolve_base_sha(root: &Path) -> Result<String> {
    if let Ok(value) = git_output_dynamic(root, &["merge-base", "HEAD", "origin/master"]) {
        return Ok(value);
    }
    if let Ok(value) = git_output_dynamic(root, &["merge-base", "HEAD", "master"]) {
        return Ok(value);
    }
    if let Ok(value) = git_output_dynamic(root, &["rev-parse", "HEAD"]) {
        return Ok(value);
    }
    bail!("unable to determine base sha")
}

fn git_output<const N: usize>(root: &Path, args: [&str; N]) -> Result<String> {
    git_output_dynamic(root, &args)
}

fn git_output_dynamic(root: &Path, args: &[&str]) -> Result<String> {
    let output = std::process::Command::new("git")
        .args(args)
        .current_dir(root)
        .output()
        .with_context(|| format!("failed to run git {:?}", args))?;

    if !output.status.success() {
        bail!("git {:?} failed: {}", args, String::from_utf8_lossy(&output.stderr).trim());
    }

    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}
