use color_eyre::eyre::{Context, Result};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct QueueSnapshot {
    pub generated_at: String,
    pub prs: Vec<PrSnapshot>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct PrSnapshot {
    pub number: u64,
    pub draft: bool,
    pub merged: bool,
    pub head_sha: String,
    pub base_sha: String,
    pub labels: Vec<String>,
    pub status_rollup: StatusRollup,
    #[serde(default)]
    pub receipts: Vec<ReceiptRef>,
}

#[derive(Debug, Serialize, Deserialize, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum StatusRollup {
    Green,
    Red,
    Pending,
    Unknown,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ReceiptRef {
    pub kind: String,
    pub path: PathBuf,
    #[serde(default)]
    pub head_sha: Option<String>,
    #[serde(default)]
    pub base_sha: Option<String>,
    #[serde(default)]
    pub valid: Option<bool>,
}

#[derive(Debug, Deserialize)]
struct GitHubEvent {
    pull_request: Option<GitHubPullRequest>,
}

#[derive(Debug, Deserialize)]
struct GitHubPullRequest {
    number: u64,
    draft: bool,
    merged: Option<bool>,
    head: GitRef,
    base: GitRef,
    labels: Vec<GitLabel>,
}

#[derive(Debug, Deserialize)]
struct GitRef {
    sha: String,
}

#[derive(Debug, Deserialize)]
struct GitLabel {
    name: String,
}

pub fn run(out: PathBuf) -> Result<()> {
    let snapshot = snapshot_from_github_event_or_empty()?;
    write_snapshot(&out, &snapshot)
}

fn snapshot_from_github_event_or_empty() -> Result<QueueSnapshot> {
    let generated_at = chrono::Utc::now().to_rfc3339();
    let event_path = std::env::var_os("GITHUB_EVENT_PATH").map(PathBuf::from);

    let prs =
        if let Some(path) = event_path { read_single_pr_from_event(&path)? } else { Vec::new() };

    Ok(QueueSnapshot { generated_at, prs })
}

fn read_single_pr_from_event(path: &Path) -> Result<Vec<PrSnapshot>> {
    let raw = fs::read_to_string(path)
        .with_context(|| format!("failed to read GitHub event payload at {}", path.display()))?;
    let event: GitHubEvent = serde_json::from_str(&raw)
        .with_context(|| format!("failed to parse GitHub event payload at {}", path.display()))?;

    let Some(pr) = event.pull_request else {
        return Ok(Vec::new());
    };

    let labels = pr.labels.into_iter().map(|l| l.name).collect::<Vec<_>>();
    Ok(vec![PrSnapshot {
        number: pr.number,
        draft: pr.draft,
        merged: pr.merged.unwrap_or(false),
        head_sha: pr.head.sha,
        base_sha: pr.base.sha,
        labels,
        status_rollup: StatusRollup::Unknown,
        receipts: Vec::new(),
    }])
}

fn write_snapshot(out: &Path, snapshot: &QueueSnapshot) -> Result<()> {
    if let Some(parent) = out.parent() {
        fs::create_dir_all(parent).with_context(|| {
            format!("failed to create snapshot output dir {}", parent.display())
        })?;
    }

    let json =
        serde_json::to_string_pretty(snapshot).context("failed to serialize queue snapshot")?;
    fs::write(out, format!("{json}\n"))
        .with_context(|| format!("failed to write queue snapshot to {}", out.display()))?;
    println!("Wrote queue snapshot to {}", out.display());
    Ok(())
}
