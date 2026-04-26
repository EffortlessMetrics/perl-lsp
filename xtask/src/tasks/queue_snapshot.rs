use color_eyre::eyre::{Context, Result};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use std::process::Command;

use crate::utils::project_root;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueueSnapshot {
    pub generated_at: String,
    pub prs: Vec<PullRequestSnapshot>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PullRequestSnapshot {
    pub number: u64,
    pub draft: bool,
    pub merged_at: Option<String>,
    pub head_sha: String,
    pub base_sha: String,
    pub labels: Vec<String>,
    pub status_rollup: Option<String>,
}

#[derive(Debug, Deserialize)]
struct GhLabel {
    name: String,
}

#[derive(Debug, Deserialize)]
struct GhPr {
    number: u64,
    #[serde(default)]
    #[serde(rename = "isDraft")]
    is_draft: bool,
    #[serde(rename = "mergedAt")]
    merged_at: Option<String>,
    #[serde(rename = "headRefOid")]
    head_ref_oid: String,
    #[serde(rename = "baseRefOid")]
    base_ref_oid: String,
    #[serde(default)]
    labels: Vec<GhLabel>,
    #[serde(rename = "statusCheckRollup")]
    status_check_rollup: Option<Vec<serde_json::Value>>,
}

pub fn run(out: PathBuf) -> Result<()> {
    let root = project_root()?;
    std::env::set_current_dir(&root).context("failed to change to workspace root")?;

    let prs = fetch_prs_from_gh()?;
    let snapshot = QueueSnapshot { generated_at: chrono::Utc::now().to_rfc3339(), prs };

    if let Some(parent) = out.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("failed to create output directory {}", parent.display()))?;
    }
    let encoded =
        serde_json::to_vec_pretty(&snapshot).context("failed to encode queue snapshot")?;
    fs::write(&out, encoded).with_context(|| format!("failed to write {}", out.display()))?;
    println!("wrote queue snapshot to {}", out.display());
    Ok(())
}

fn fetch_prs_from_gh() -> Result<Vec<PullRequestSnapshot>> {
    let output = Command::new("gh")
        .args([
            "pr",
            "list",
            "--limit",
            "200",
            "--json",
            "number,isDraft,mergedAt,headRefOid,baseRefOid,labels,statusCheckRollup",
        ])
        .output();

    let output = match output {
        Ok(out) if out.status.success() => out,
        _ => return Ok(Vec::new()),
    };

    let prs: Vec<GhPr> = serde_json::from_slice(&output.stdout)
        .context("failed to decode gh pr list JSON; expected GitHub CLI payload")?;

    let mapped = prs
        .into_iter()
        .map(|pr| PullRequestSnapshot {
            number: pr.number,
            draft: pr.is_draft,
            merged_at: pr.merged_at,
            head_sha: pr.head_ref_oid,
            base_sha: pr.base_ref_oid,
            labels: pr.labels.into_iter().map(|label| label.name).collect(),
            status_rollup: summarize_rollup(pr.status_check_rollup.as_ref()),
        })
        .collect();

    Ok(mapped)
}

fn summarize_rollup(rollup: Option<&Vec<serde_json::Value>>) -> Option<String> {
    let items = rollup?;
    if items.is_empty() {
        return Some("PENDING".to_string());
    }

    let mut has_failure = false;
    let mut has_pending = false;
    for item in items {
        let conclusion =
            item.get("conclusion").and_then(serde_json::Value::as_str).map(str::to_uppercase);
        let state = item.get("state").and_then(serde_json::Value::as_str).map(str::to_uppercase);
        if let Some(conclusion) = conclusion {
            if conclusion == "FAILURE" || conclusion == "TIMED_OUT" || conclusion == "CANCELLED" {
                has_failure = true;
            } else if conclusion == "NEUTRAL" || conclusion == "SKIPPED" || conclusion == "SUCCESS"
            {
            } else {
                has_pending = true;
            }
        } else if let Some(state) = state {
            if state == "FAILURE" || state == "ERROR" {
                has_failure = true;
            } else if state == "PENDING" || state == "IN_PROGRESS" || state == "QUEUED" {
                has_pending = true;
            }
        } else {
            has_pending = true;
        }
    }

    if has_failure {
        Some("RED".to_string())
    } else if has_pending {
        Some("PENDING".to_string())
    } else {
        Some("GREEN".to_string())
    }
}
