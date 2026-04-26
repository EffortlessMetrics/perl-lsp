use color_eyre::eyre::{Context, Result, bail};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

const MERGE_READY_CHECK: &str = "merge-readiness";
const SCHEMA_VERSION: &str = "1";

#[derive(clap::Subcommand, Debug, Clone)]
pub enum MergeReadyCommand {
    /// Emit a merge-readiness receipt for a PR.
    Emit {
        /// Pull request number.
        #[arg(long)]
        pr: u64,
        /// Output path for the receipt JSON.
        #[arg(long)]
        receipt: PathBuf,
    },
    /// Verify merge readiness from either a live PR view or a fixture receipt.
    Verify {
        /// Pull request number.
        #[arg(long)]
        pr: Option<u64>,
        /// Optional fixture receipt JSON path.
        #[arg(long)]
        fixture: Option<PathBuf>,
    },
    /// Reconcile merge-ready labels for open PRs.
    Reconcile {
        /// Apply label removals instead of dry-run reporting.
        #[arg(long)]
        apply: bool,
        /// Force dry-run mode even when --apply is not passed.
        #[arg(long)]
        dry_run: bool,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MergeReadinessReceipt {
    pub check: String,
    pub schema_version: String,
    pub event: String,
    pub pr: u64,
    pub head_sha: String,
    pub base_sha: String,
    pub gate_graph_version: String,
    pub required_checks: Vec<RequiredCheckEvidence>,
    pub review_evidence: ReviewEvidence,
    pub blocker_labels_absent: bool,
    pub verdict: String,
    pub expires_when: ExpiresWhen,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RequiredCheckEvidence {
    pub name: String,
    pub status: String,
    pub conclusion: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReviewEvidence {
    pub approved: bool,
    pub approvals: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExpiresWhen {
    pub head_sha_changes: bool,
    pub base_sha_changes: bool,
    pub gate_graph_version_changes: bool,
}

#[derive(Debug, Deserialize)]
struct RequiredChecksPolicy {
    required_checks: Vec<String>,
    #[serde(default)]
    blocker_labels: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct PrView {
    number: u64,
    #[serde(rename = "headRefOid")]
    head_ref_oid: String,
    #[serde(rename = "baseRefOid")]
    base_ref_oid: String,
    labels: Vec<Label>,
    reviews: Vec<Review>,
    #[serde(rename = "statusCheckRollup")]
    status_check_rollup: Vec<StatusRollup>,
}

#[derive(Debug, Deserialize)]
struct Label {
    name: String,
}

#[derive(Debug, Deserialize)]
struct Review {
    state: String,
}

#[derive(Debug, Deserialize)]
struct StatusRollup {
    #[serde(rename = "__typename")]
    type_name: String,
    #[serde(rename = "status", default)]
    status: String,
    #[serde(rename = "conclusion", default)]
    conclusion: String,
    #[serde(rename = "name", default)]
    name: String,
    #[serde(rename = "context", default)]
    context: String,
}

pub fn run(command: MergeReadyCommand) -> Result<()> {
    match command {
        MergeReadyCommand::Emit { pr, receipt } => emit(pr, &receipt),
        MergeReadyCommand::Verify { pr, fixture } => verify(pr, fixture.as_deref()),
        MergeReadyCommand::Reconcile { apply, dry_run } => reconcile(apply && !dry_run),
    }
}

fn emit(pr: u64, receipt_path: &Path) -> Result<()> {
    let receipt = build_receipt_from_live_pr(pr)?;
    if let Some(parent) = receipt_path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("failed to create {}", parent.display()))?;
    }
    let json = serde_json::to_string_pretty(&receipt)?;
    fs::write(receipt_path, json)
        .with_context(|| format!("failed to write {}", receipt_path.display()))?;
    println!("{}", receipt.verdict);
    Ok(())
}

fn verify(pr: Option<u64>, fixture: Option<&Path>) -> Result<()> {
    let receipt = if let Some(fixture_path) = fixture {
        let raw = fs::read_to_string(fixture_path)
            .with_context(|| format!("failed to read fixture {}", fixture_path.display()))?;
        serde_json::from_str::<MergeReadinessReceipt>(&raw)
            .with_context(|| format!("invalid fixture {}", fixture_path.display()))?
    } else {
        let number = pr.ok_or_else(|| {
            color_eyre::eyre::eyre!("--pr is required when --fixture is not provided")
        })?;
        build_receipt_from_live_pr(number)?
    };

    let normalized = normalize_verdict(&receipt.verdict);
    println!("{normalized}");
    Ok(())
}

fn reconcile(apply: bool) -> Result<()> {
    let mode = if apply { "apply" } else { "dry-run" };
    println!("merge-ready reconcile mode: {mode}");
    let output =
        run_gh(["pr", "list", "--search", "is:open label:merge-ready", "--json", "number"])?;

    #[derive(Deserialize)]
    struct PrNum {
        number: u64,
    }

    let prs: Vec<PrNum> =
        serde_json::from_str(&output).context("failed to parse gh pr list JSON")?;
    for pr in prs {
        let receipt = build_receipt_from_live_pr(pr.number)?;
        let verdict = normalize_verdict(&receipt.verdict);
        if verdict == "valid" {
            continue;
        }

        println!("pr #{}: would remove merge-ready ({verdict})", pr.number);
        if apply {
            run_gh(["pr", "edit", &pr.number.to_string(), "--remove-label", "merge-ready"])?;
            let body = format!(
                "Removing merge-ready: `{}` for head `{}` base `{}` gate graph `{}`.",
                verdict, receipt.head_sha, receipt.base_sha, receipt.gate_graph_version
            );
            run_gh(["pr", "comment", &pr.number.to_string(), "--body", &body])?;
        }
    }

    Ok(())
}

fn build_receipt_from_live_pr(pr: u64) -> Result<MergeReadinessReceipt> {
    let policy = read_policy()?;
    let gate_graph_version = compute_gate_graph_version()?;
    let pr_view = fetch_pr(pr)?;

    let mut status_by_name = std::collections::BTreeMap::new();
    for rollup in pr_view.status_check_rollup {
        let check_name = if rollup.type_name == "CheckRun" {
            rollup.name.clone()
        } else {
            rollup.context.clone()
        };
        if !check_name.is_empty() {
            status_by_name.insert(check_name, rollup);
        }
    }

    let mut required_checks = Vec::new();
    let mut blocked = false;
    let mut missing = false;
    for check in &policy.required_checks {
        if let Some(entry) = status_by_name.get(check) {
            required_checks.push(RequiredCheckEvidence {
                name: check.clone(),
                status: entry.status.clone(),
                conclusion: entry.conclusion.clone(),
            });
            if entry.conclusion.to_ascii_lowercase() != "success" {
                blocked = true;
            }
        } else {
            missing = true;
            required_checks.push(RequiredCheckEvidence {
                name: check.clone(),
                status: "MISSING".to_string(),
                conclusion: "MISSING".to_string(),
            });
        }
    }

    let approval_count =
        pr_view.reviews.iter().filter(|r| r.state.eq_ignore_ascii_case("APPROVED")).count() as u64;

    let labels: BTreeSet<String> = pr_view.labels.into_iter().map(|label| label.name).collect();
    let blocker_labels_absent = policy.blocker_labels.iter().all(|label| !labels.contains(label));

    let verdict = if missing {
        "missing"
    } else if blocked || !blocker_labels_absent || approval_count == 0 {
        "blocked"
    } else {
        "valid"
    };

    Ok(MergeReadinessReceipt {
        check: MERGE_READY_CHECK.to_string(),
        schema_version: SCHEMA_VERSION.to_string(),
        event: "pull_request".to_string(),
        pr: pr_view.number,
        head_sha: pr_view.head_ref_oid,
        base_sha: pr_view.base_ref_oid,
        gate_graph_version,
        required_checks,
        review_evidence: ReviewEvidence { approved: approval_count > 0, approvals: approval_count },
        blocker_labels_absent,
        verdict: verdict.to_string(),
        expires_when: ExpiresWhen {
            head_sha_changes: true,
            base_sha_changes: true,
            gate_graph_version_changes: true,
        },
    })
}

fn fetch_pr(pr: u64) -> Result<PrView> {
    let output = run_gh([
        "pr",
        "view",
        &pr.to_string(),
        "--json",
        "number,headRefOid,baseRefOid,labels,reviews,statusCheckRollup",
    ])?;
    serde_json::from_str(&output).context("failed to parse gh pr view JSON")
}

fn read_policy() -> Result<RequiredChecksPolicy> {
    let root = crate::utils::project_root()?;
    let path = root.join(".ci/policies/required-checks.toml");
    let raw = fs::read_to_string(&path)
        .with_context(|| format!("failed to read required checks policy: {}", path.display()))?;
    toml::from_str(&raw).context("invalid required-checks.toml")
}

fn compute_gate_graph_version() -> Result<String> {
    let root = crate::utils::project_root()?;
    let mut files = Vec::new();

    collect_files(&root.join(".ci/policies"), &mut files)?;
    collect_files(&root.join(".ci/gates.d"), &mut files)?;
    collect_files(&root.join(".github/workflows"), &mut files)?;

    files.sort();

    let mut hasher = Fnv1a64::default();
    for file in files {
        let rel = file
            .strip_prefix(&root)
            .with_context(|| format!("failed to strip workspace prefix for {}", file.display()))?;
        hasher.write(rel.to_string_lossy().as_bytes());
        hasher.write(b"\n");
        let content = fs::read(&file)
            .with_context(|| format!("failed to read gate graph file {}", file.display()))?;
        hasher.write(&content);
        hasher.write(b"\n");
    }

    Ok(format!("fnv1a64:{:016x}", hasher.finish()))
}

fn collect_files(root: &Path, files: &mut Vec<PathBuf>) -> Result<()> {
    if !root.exists() {
        return Ok(());
    }
    for entry in walkdir::WalkDir::new(root) {
        let entry = entry.with_context(|| format!("failed to walk {}", root.display()))?;
        if entry.file_type().is_file() {
            files.push(entry.path().to_path_buf());
        }
    }
    Ok(())
}

fn run_gh<const N: usize>(args: [&str; N]) -> Result<String> {
    let output = Command::new("gh").args(args).output().context("failed to execute gh CLI")?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        bail!("gh command failed: {stderr}");
    }
    let stdout = String::from_utf8(output.stdout).context("gh output was not valid UTF-8")?;
    Ok(stdout)
}

fn normalize_verdict(verdict: &str) -> &str {
    match verdict {
        "valid" | "stale_head" | "stale_base" | "stale_gate_graph" | "blocked" | "missing" => {
            verdict
        }
        _ => "blocked",
    }
}

#[derive(Default)]
struct Fnv1a64 {
    state: u64,
}

impl Fnv1a64 {
    fn write(&mut self, bytes: &[u8]) {
        if self.state == 0 {
            self.state = 0xcbf29ce484222325;
        }
        for byte in bytes {
            self.state ^= u64::from(*byte);
            self.state = self.state.wrapping_mul(0x100000001b3);
        }
    }

    fn finish(&self) -> u64 {
        self.state
    }
}
