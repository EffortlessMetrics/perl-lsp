use chrono::Utc;
use color_eyre::eyre::{Context, Result};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

use crate::utils::project_root;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReceiptRef {
    pub kind: String,
    pub path: String,
    pub head_sha: Option<String>,
    pub base_sha: Option<String>,
    pub valid: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PullRequestSnapshot {
    pub number: u64,
    #[serde(default)]
    pub draft: bool,
    #[serde(default)]
    pub labels: Vec<String>,
    pub head_sha: String,
    pub base_sha: String,
    #[serde(default = "default_status_rollup")]
    pub status_rollup: String,
    #[serde(default)]
    pub receipts: Vec<ReceiptRef>,
}

fn default_status_rollup() -> String {
    "unknown".to_string()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueueSnapshot {
    pub schema_version: u32,
    pub generated_at: String,
    #[serde(default)]
    pub prs: Vec<PullRequestSnapshot>,
}

pub fn run(out: PathBuf) -> Result<()> {
    let root = project_root()?;
    let receipts = collect_receipts(&root.join("target/receipts"))?;

    let snapshot =
        QueueSnapshot { schema_version: 1, generated_at: Utc::now().to_rfc3339(), prs: receipts };

    if let Some(parent) = out.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("failed to create output directory: {}", parent.display()))?;
    }

    let json = serde_json::to_string_pretty(&snapshot).context("failed to serialize snapshot")?;
    fs::write(&out, json)
        .with_context(|| format!("failed to write snapshot: {}", out.display()))?;
    println!("wrote queue snapshot to {}", out.display());
    Ok(())
}

fn collect_receipts(receipts_dir: &Path) -> Result<Vec<PullRequestSnapshot>> {
    if !receipts_dir.exists() {
        return Ok(Vec::new());
    }

    let mut refs = Vec::new();
    for entry in fs::read_dir(receipts_dir)
        .with_context(|| format!("failed to read receipts directory: {}", receipts_dir.display()))?
    {
        let entry = entry?;
        let path = entry.path();
        if path.extension().and_then(|s| s.to_str()) != Some("json") {
            continue;
        }

        let raw = fs::read_to_string(&path)
            .with_context(|| format!("failed to read receipt {}", path.display()))?;
        let value: serde_json::Value = match serde_json::from_str(&raw) {
            Ok(v) => v,
            Err(_) => continue,
        };

        let number = value.get("pr_number").and_then(|v| v.as_u64());
        let head_sha = value.get("head_sha").and_then(|v| v.as_str()).map(ToOwned::to_owned);
        let base_sha = value.get("base_sha").and_then(|v| v.as_str()).map(ToOwned::to_owned);

        if let (Some(number), Some(head_sha), Some(base_sha)) = (number, head_sha, base_sha) {
            refs.push(PullRequestSnapshot {
                number,
                draft: false,
                labels: Vec::new(),
                head_sha,
                base_sha,
                status_rollup: default_status_rollup(),
                receipts: Vec::new(),
            });
        }
    }

    Ok(refs)
}
