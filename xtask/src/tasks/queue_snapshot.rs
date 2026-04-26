use color_eyre::eyre::{Context, Result};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use std::process::Command;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct QueueSnapshot {
    pub schema_version: u32,
    pub generated_by: String,
    pub pull_requests: Vec<PullRequestFacts>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PullRequestFacts {
    pub number: u64,
    #[serde(default)]
    pub draft: bool,
    #[serde(default)]
    pub merged: bool,
    #[serde(default)]
    pub labels: Vec<String>,
    pub head_sha: String,
    pub base_sha: String,
    #[serde(default)]
    pub status_rollup: String,
}

#[derive(Debug, Deserialize)]
struct GhPr {
    number: u64,
    #[serde(rename = "isDraft")]
    is_draft: bool,
    #[serde(rename = "isMerged")]
    is_merged: bool,
    labels: Vec<GhLabel>,
    #[serde(rename = "headRefOid")]
    head_ref_oid: String,
    #[serde(rename = "baseRefOid")]
    base_ref_oid: String,
    #[serde(rename = "statusCheckRollup")]
    status_check_rollup: Option<Vec<GhStatusContext>>,
}

#[derive(Debug, Deserialize)]
struct GhLabel {
    name: String,
}

#[derive(Debug, Deserialize)]
struct GhStatusContext {
    #[serde(rename = "__typename")]
    type_name: String,
    #[serde(default)]
    state: Option<String>,
    #[serde(default)]
    conclusion: Option<String>,
}

pub fn run(out: PathBuf) -> Result<()> {
    let snapshot = collect_snapshot().unwrap_or_default();
    if let Some(parent) = out.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("failed to create output directory {}", parent.display()))?;
    }
    let json = serde_json::to_string_pretty(&snapshot)?;
    fs::write(&out, json).with_context(|| format!("failed to write {}", out.display()))?;
    Ok(())
}

fn collect_snapshot() -> Result<QueueSnapshot> {
    let output = Command::new("gh")
        .args([
            "pr",
            "list",
            "--state",
            "all",
            "--limit",
            "200",
            "--json",
            "number,isDraft,isMerged,labels,headRefOid,baseRefOid,statusCheckRollup",
        ])
        .output()
        .context("failed to invoke gh")?;

    if !output.status.success() {
        return Ok(QueueSnapshot {
            schema_version: 1,
            generated_by: "queue-snapshot-dry-run".to_string(),
            pull_requests: Vec::new(),
        });
    }

    let raw = String::from_utf8(output.stdout).context("gh output was not utf-8")?;
    let gh_prs: Vec<GhPr> = serde_json::from_str(&raw).context("failed to parse gh PR payload")?;

    let pull_requests = gh_prs
        .into_iter()
        .map(|pr| PullRequestFacts {
            number: pr.number,
            draft: pr.is_draft,
            merged: pr.is_merged,
            labels: pr.labels.into_iter().map(|label| label.name).collect(),
            head_sha: pr.head_ref_oid,
            base_sha: pr.base_ref_oid,
            status_rollup: normalize_rollup(&pr.status_check_rollup),
        })
        .collect();

    Ok(QueueSnapshot {
        schema_version: 1,
        generated_by: "queue-snapshot-dry-run".to_string(),
        pull_requests,
    })
}

fn normalize_rollup(contexts: &Option<Vec<GhStatusContext>>) -> String {
    let Some(contexts) = contexts else {
        return "unknown".to_string();
    };

    let mut any_failure = false;
    let mut any_pending = false;
    let mut any_success = false;

    for context in contexts {
        let value = if context.type_name == "CheckRun" {
            context.conclusion.as_deref().unwrap_or("PENDING")
        } else {
            context.state.as_deref().unwrap_or("PENDING")
        };
        match value {
            "FAILURE" | "ERROR" | "TIMED_OUT" | "CANCELLED" => any_failure = true,
            "SUCCESS" => any_success = true,
            _ => any_pending = true,
        }
    }

    if any_failure {
        "red".to_string()
    } else if any_pending {
        "pending".to_string()
    } else if any_success {
        "green".to_string()
    } else {
        "unknown".to_string()
    }
}
